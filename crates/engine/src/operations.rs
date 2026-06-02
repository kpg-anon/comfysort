//! Journaled, reversible filesystem operations.
//!
//! Every mutation records an intent record, performs the operation, then records
//! a result record (append-only JSONL). A session-local stack powers multi-step
//! undo. This is the only module that mutates the filesystem.

use crate::domain::CollisionPolicy;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationKind {
    Move,
    Copy,
    DeleteFolder,
    UndoMove,
    UndoCopy,
    UndoDeleteFolder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationStatus {
    Intent,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalRecord {
    pub kind: OperationKind,
    pub status: OperationStatus,
    pub source_path: PathBuf,
    pub resolved_path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub message: String,
}

/// A completed operation kept on the undo stack.
#[derive(Debug, Clone)]
pub struct CompletedOp {
    pub kind: OperationKind,
    /// Where the file started (inbox path for a move/trash).
    pub source_path: PathBuf,
    /// Where it ended up (the collision-resolved destination).
    pub resolved_path: PathBuf,
}

pub struct OperationEngine {
    journal_path: PathBuf,
    completed: Vec<CompletedOp>,
}

impl OperationEngine {
    pub fn new(journal_path: PathBuf) -> Self {
        Self {
            journal_path,
            completed: Vec::new(),
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.completed.is_empty()
    }

    /// Move `source` into `dest_dir`, journaled and undoable. Trash is just a
    /// move into the trash directory, so undo restores it the same way.
    pub fn move_file(&mut self, source: &Path, dest_dir: &Path) -> anyhow::Result<PathBuf> {
        let resolved = self.plan(source, dest_dir)?;
        self.record(OperationKind::Move, OperationStatus::Intent, source, &resolved);
        relocate(source, &resolved)?;
        self.record(
            OperationKind::Move,
            OperationStatus::Succeeded,
            source,
            &resolved,
        );
        self.completed.push(CompletedOp {
            kind: OperationKind::Move,
            source_path: source.to_path_buf(),
            resolved_path: resolved.clone(),
        });
        Ok(resolved)
    }

    /// Copy `source` into `dest_dir`, leaving the original in place.
    pub fn copy_file(&mut self, source: &Path, dest_dir: &Path) -> anyhow::Result<PathBuf> {
        let resolved = self.plan(source, dest_dir)?;
        self.record(OperationKind::Copy, OperationStatus::Intent, source, &resolved);
        fs::copy(source, &resolved)?;
        self.record(
            OperationKind::Copy,
            OperationStatus::Succeeded,
            source,
            &resolved,
        );
        self.completed.push(CompletedOp {
            kind: OperationKind::Copy,
            source_path: source.to_path_buf(),
            resolved_path: resolved.clone(),
        });
        Ok(resolved)
    }

    /// Move a folder into `trash_dir`, journaled and undoable. A plain
    /// filesystem rename so undo can rename it straight back; collisions inside
    /// trash get the Windows-Explorer suffix treatment. Returns the resolved
    /// destination inside trash.
    pub fn delete_folder(
        &mut self,
        source: &Path,
        trash_dir: &Path,
    ) -> anyhow::Result<PathBuf> {
        let file_name = source
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("source path has no file name"))?;
        fs::create_dir_all(trash_dir)?;
        let resolved = resolve_collision(trash_dir.join(file_name), CollisionPolicy::Rename)?;
        self.record(
            OperationKind::DeleteFolder,
            OperationStatus::Intent,
            source,
            &resolved,
        );
        fs::rename(source, &resolved)?;
        self.record(
            OperationKind::DeleteFolder,
            OperationStatus::Succeeded,
            source,
            &resolved,
        );
        self.completed.push(CompletedOp {
            kind: OperationKind::DeleteFolder,
            source_path: source.to_path_buf(),
            resolved_path: resolved.clone(),
        });
        Ok(resolved)
    }

    /// Reverse the most recent completed operation. Returns the completed op so
    /// the caller can decide how to update the inbox (a reversed move/trash
    /// restores a file to its source; a reversed copy removes the duplicate).
    pub fn undo_last(&mut self) -> anyhow::Result<CompletedOp> {
        let last = self
            .completed
            .pop()
            .ok_or_else(|| anyhow::anyhow!("nothing to undo"))?;
        match last.kind {
            OperationKind::Move => {
                if last.source_path.exists() {
                    anyhow::bail!("cannot undo move: a file already exists at the original path");
                }
                relocate(&last.resolved_path, &last.source_path)?;
                self.record(
                    OperationKind::UndoMove,
                    OperationStatus::Succeeded,
                    &last.resolved_path,
                    &last.source_path,
                );
            }
            OperationKind::Copy => {
                fs::remove_file(&last.resolved_path)?;
                self.record(
                    OperationKind::UndoCopy,
                    OperationStatus::Succeeded,
                    &last.resolved_path,
                    &last.resolved_path,
                );
            }
            OperationKind::DeleteFolder => {
                if last.source_path.exists() {
                    anyhow::bail!(
                        "cannot undo delete: the original folder path already exists"
                    );
                }
                fs::rename(&last.resolved_path, &last.source_path)?;
                self.record(
                    OperationKind::UndoDeleteFolder,
                    OperationStatus::Succeeded,
                    &last.resolved_path,
                    &last.source_path,
                );
            }
            OperationKind::UndoMove
            | OperationKind::UndoCopy
            | OperationKind::UndoDeleteFolder => {
                anyhow::bail!("undo records are not themselves undoable");
            }
        }
        Ok(last)
    }

    /// Resolve the final destination path, creating the directory and applying
    /// the collision policy (rename by default).
    fn plan(&self, source: &Path, dest_dir: &Path) -> anyhow::Result<PathBuf> {
        let file_name = source
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("source path has no file name"))?;
        fs::create_dir_all(dest_dir)?;
        resolve_collision(dest_dir.join(file_name), CollisionPolicy::Rename)
    }

    fn record(
        &self,
        kind: OperationKind,
        status: OperationStatus,
        source: &Path,
        resolved: &Path,
    ) {
        let record = JournalRecord {
            kind,
            status,
            source_path: source.to_path_buf(),
            resolved_path: resolved.to_path_buf(),
            created_at: Utc::now(),
            message: format!("{:?} {:?}", kind, status),
        };
        // Journaling is best-effort durability; a write failure must not crash a
        // sort session, but we surface it to the debug channel.
        if let Err(err) = append_journal(&self.journal_path, &record) {
            eprintln!("comfysort: journal write failed: {err}");
        }
    }
}

fn append_journal(journal_path: &Path, record: &JournalRecord) -> anyhow::Result<()> {
    if let Some(parent) = journal_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(journal_path)?;
    serde_json::to_writer(&mut file, record)?;
    writeln!(file)?;
    Ok(())
}

/// Move a file, transparently handling cross-volume relocation. `fs::rename`
/// only works within one filesystem; on a cross-device error we fall back to
/// copy → verify size → delete source (source kept until the copy is verified).
fn relocate(from: &Path, to: &Path) -> anyhow::Result<()> {
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(err) if is_cross_device_err(&err) => cross_device_move(from, to),
        Err(err) => Err(anyhow::Error::from(err)
            .context(format!("renaming {} -> {}", from.display(), to.display()))),
    }
}

fn cross_device_move(from: &Path, to: &Path) -> anyhow::Result<()> {
    let copied = fs::copy(from, to)?;
    let source_len = fs::metadata(from).map(|m| m.len()).unwrap_or(copied);
    if copied != source_len {
        let _ = fs::remove_file(to);
        anyhow::bail!(
            "cross-device copy of {} was incomplete ({copied} of {source_len} bytes); source left intact",
            from.display()
        );
    }
    fs::remove_file(from)?;
    Ok(())
}

fn is_cross_device_err(err: &std::io::Error) -> bool {
    #[cfg(windows)]
    {
        err.raw_os_error() == Some(17) // ERROR_NOT_SAME_DEVICE
    }
    #[cfg(unix)]
    {
        err.raw_os_error() == Some(libc::EXDEV)
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = err;
        false
    }
}

/// Resolve a collision per policy. Default `Rename` produces Windows-Explorer
/// `name (2).ext`, `name (3).ext`, … suffixes and never clobbers.
pub fn resolve_collision(requested: PathBuf, policy: CollisionPolicy) -> anyhow::Result<PathBuf> {
    if !requested.exists() {
        return Ok(requested);
    }
    match policy {
        CollisionPolicy::Overwrite => Ok(requested),
        CollisionPolicy::Skip => anyhow::bail!("destination already exists"),
        CollisionPolicy::Rename => renamed_candidate(&requested),
    }
}

fn renamed_candidate(requested: &Path) -> anyhow::Result<PathBuf> {
    let parent = requested.parent().unwrap_or_else(|| Path::new(""));
    let stem = requested
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("requested path has no file stem"))?;
    let ext = requested.extension().and_then(|e| e.to_str());
    for index in 2..100_000 {
        let name = match ext {
            Some(ext) => format!("{stem} ({index}).{ext}"),
            None => format!("{stem} ({index})"),
        };
        let candidate = parent.join(name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    anyhow::bail!("could not find an available rename candidate")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn move_then_undo_restores_source() {
        let dir = tempdir().unwrap();
        let inbox = dir.path().join("inbox");
        let keep = dir.path().join("keep");
        fs::create_dir_all(&inbox).unwrap();
        let src = inbox.join("a.jpg");
        fs::write(&src, b"img").unwrap();

        let mut engine = OperationEngine::new(dir.path().join("j.jsonl"));
        let resolved = engine.move_file(&src, &keep).unwrap();
        assert!(!src.exists());
        assert!(resolved.exists());

        engine.undo_last().unwrap();
        assert!(src.exists());
        assert!(!resolved.exists());
    }

    #[test]
    fn copy_keeps_source_and_undo_removes_dup() {
        let dir = tempdir().unwrap();
        let inbox = dir.path().join("inbox");
        let keep = dir.path().join("keep");
        fs::create_dir_all(&inbox).unwrap();
        let src = inbox.join("a.jpg");
        fs::write(&src, b"img").unwrap();

        let mut engine = OperationEngine::new(dir.path().join("j.jsonl"));
        let resolved = engine.copy_file(&src, &keep).unwrap();
        assert!(src.exists());
        assert!(resolved.exists());

        engine.undo_last().unwrap();
        assert!(src.exists());
        assert!(!resolved.exists());
    }

    #[test]
    fn delete_folder_moves_into_trash_and_undo_restores_it() {
        let dir = tempdir().unwrap();
        let group = dir.path().join("group");
        let stray = group.join("stray");
        let trash = dir.path().join(".trash");
        fs::create_dir_all(&stray).unwrap();
        fs::write(stray.join("child.txt"), b"contents").unwrap();

        let mut engine = OperationEngine::new(dir.path().join("j.jsonl"));
        let resolved = engine.delete_folder(&stray, &trash).unwrap();

        assert!(!stray.exists(), "original folder must be gone");
        assert!(resolved.exists(), "trashed copy must exist");
        assert!(
            resolved.join("child.txt").exists(),
            "nested contents follow the rename"
        );

        engine.undo_last().unwrap();
        assert!(stray.exists(), "undo restores the folder");
        assert!(stray.join("child.txt").exists());
        assert!(!resolved.exists(), "trash slot is cleared");
    }

    #[test]
    fn collision_uses_explorer_suffixes() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), b"x").unwrap();
        fs::write(dir.path().join("a (2).jpg"), b"x").unwrap();
        let resolved =
            resolve_collision(dir.path().join("a.jpg"), CollisionPolicy::Rename).unwrap();
        assert_eq!(resolved, dir.path().join("a (3).jpg"));
    }
}
