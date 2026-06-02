//! In-memory session state: the roots, the live destination list, and the
//! operation engine. Wrapped in a `Mutex` and `manage`d by Tauri.

use crate::destinations::{count_media, scan_destinations};
use crate::domain::{
    DestinationDto, FolderEntry, FolderListing, MediaItemDto, OpKind, OpOutcome, STATE_DIR,
    SessionView, journal_path, trash_dir,
};
use crate::logging::log;
use crate::media::scan_inbox;
use crate::operations::{CompletedOp, OperationEngine, OperationKind};
use crate::search;
use std::path::{Path, PathBuf};

pub struct Session {
    input: PathBuf,
    output: PathBuf,
    destinations: Vec<DestinationDto>,
    engine: OperationEngine,
}

impl Session {
    /// Open a session against the given roots, scanning inbox + destinations.
    pub fn open(input: PathBuf, output: PathBuf) -> anyhow::Result<(Self, SessionView)> {
        let inbox = scan_inbox(&input)?;
        let destinations = scan_destinations(&output)?;
        let engine = OperationEngine::new(journal_path(&output));

        // Session-open diagnostic banner: exactly what the scanner found this
        // launch, so a reported file-disappearance can be traced to the scan.
        log(
            &output,
            &format!(
                "session open: input={} output={} destinations={} inbox={}",
                input.display(),
                output.display(),
                destinations.len(),
                inbox.len()
            ),
        );
        for item in &inbox {
            log(&output, &format!("  inbox item: {}", item.file_name));
        }

        let view = SessionView {
            input: input.to_string_lossy().into_owned(),
            output: output.to_string_lossy().into_owned(),
            inbox,
            destinations: destinations.clone(),
        };
        let session = Self {
            input,
            output,
            destinations,
            engine,
        };
        Ok((session, view))
    }

    /// Re-scan the input directory (e.g. after external changes) and return the
    /// fresh inbox. Destinations are left as-is; call after a manual refresh.
    pub fn rescan_inbox(&self) -> anyhow::Result<Vec<MediaItemDto>> {
        Ok(scan_inbox(&self.input)?)
    }

    /// List the immediate child folders of `dir` for the Navigator. `dir` is
    /// clamped to the output-root subtree so the Navigator can never escape it.
    /// Folders are sorted by media count desc, then name; `.comfysort` is hidden.
    pub fn list_folders(&self, dir: &Path) -> anyhow::Result<FolderListing> {
        let dir = self.clamp_to_output(dir);
        let mut folders = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !entry.metadata().map(|m| m.is_dir()).unwrap_or(false) {
                    continue;
                }
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if name.eq_ignore_ascii_case(STATE_DIR) {
                    continue;
                }
                folders.push(FolderEntry {
                    media_count: count_media(&path),
                    subfolder_count: count_subfolders(&path),
                    path: path.to_string_lossy().into_owned(),
                    name,
                });
            }
        }
        folders.sort_by(|a, b| b.media_count.cmp(&a.media_count).then_with(|| a.name.cmp(&b.name)));

        let parent = (dir != self.output).then(|| {
            dir.parent()
                .unwrap_or(&self.output)
                .to_string_lossy()
                .into_owned()
        });
        let rel = dir
            .strip_prefix(&self.output)
            .map(|r| r.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();

        Ok(FolderListing {
            path: dir.to_string_lossy().into_owned(),
            parent,
            rel,
            folders,
        })
    }

    /// Clamp a requested Navigator path into the output-root subtree. Anything
    /// outside (or unreadable) falls back to the output root itself.
    fn clamp_to_output(&self, dir: &Path) -> PathBuf {
        if dir == self.output || dir.starts_with(&self.output) {
            dir.to_path_buf()
        } else {
            self.output.clone()
        }
    }

    /// The destination directory bound to a hotkey, if any.
    pub fn dest_dir_for_hotkey(&self, hotkey: &str) -> Option<PathBuf> {
        self.destinations
            .iter()
            .find(|d| d.hotkey.as_deref() == Some(hotkey))
            .map(|d| PathBuf::from(&d.path))
    }

    pub fn move_item(&mut self, source: &Path, dest_dir: &Path) -> anyhow::Result<OpOutcome> {
        let resolved = self.engine.move_file(source, dest_dir)?;
        log(
            &self.output,
            &format!("move: {} -> {}", source.display(), resolved.display()),
        );
        Ok(self.outcome(
            OpKind::Move,
            format!("Moved to {}", self.label_for_dir(dest_dir)),
            source,
            &resolved,
            true,
            None,
        ))
    }

    pub fn copy_item(&mut self, source: &Path, dest_dir: &Path) -> anyhow::Result<OpOutcome> {
        let resolved = self.engine.copy_file(source, dest_dir)?;
        log(
            &self.output,
            &format!("copy: {} -> {}", source.display(), resolved.display()),
        );
        Ok(self.outcome(
            OpKind::Copy,
            format!("Copied to {}", self.label_for_dir(dest_dir)),
            source,
            &resolved,
            false,
            None,
        ))
    }

    pub fn trash_item(&mut self, source: &Path) -> anyhow::Result<OpOutcome> {
        let dir = trash_dir(&self.output);
        let resolved = self.engine.move_file(source, &dir)?;
        log(
            &self.output,
            &format!("trash: {} -> {}", source.display(), resolved.display()),
        );
        Ok(self.outcome(
            OpKind::Trash,
            "Moved to trash".to_owned(),
            source,
            &resolved,
            true,
            None,
        ))
    }

    pub fn undo(&mut self) -> anyhow::Result<OpOutcome> {
        let CompletedOp {
            kind,
            source_path,
            resolved_path,
        } = self.engine.undo_last()?;
        log(
            &self.output,
            &format!(
                "undo {:?}: {} -> {}",
                kind,
                resolved_path.display(),
                source_path.display()
            ),
        );
        match kind {
            OperationKind::Move => {
                // A reversed move/trash restores the file to the inbox.
                let restored = MediaItemDto::from_path(&source_path);
                Ok(self.outcome(
                    OpKind::Undo,
                    "Undo: restored to inbox".to_owned(),
                    &resolved_path,
                    &source_path,
                    false,
                    restored,
                ))
            }
            OperationKind::Copy => Ok(self.outcome(
                OpKind::Undo,
                "Undo: removed duplicate".to_owned(),
                &resolved_path,
                &resolved_path,
                false,
                None,
            )),
            OperationKind::DeleteFolder => Ok(self.outcome(
                OpKind::Undo,
                "Undo: restored folder".to_owned(),
                &resolved_path,
                &source_path,
                false,
                None,
            )),
            OperationKind::UndoMove
            | OperationKind::UndoCopy
            | OperationKind::UndoDeleteFolder => {
                anyhow::bail!("undo records are not themselves undoable")
            }
        }
    }

    /// Move a destination folder into trash, reversible. Refuses to delete the
    /// output root, the `.comfysort` state dir, or the trash dir itself.
    pub fn delete_folder(&mut self, path: &Path) -> anyhow::Result<OpOutcome> {
        let state = self.output.join(STATE_DIR);
        let trash = trash_dir(&self.output);
        if path == self.output {
            anyhow::bail!("refusing to delete the output root");
        }
        if path == state || path.starts_with(&state) {
            anyhow::bail!("refusing to delete the .comfysort state directory");
        }
        if path == trash {
            anyhow::bail!("refusing to delete the trash directory");
        }

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        let resolved = self.engine.delete_folder(path, &trash)?;
        log(
            &self.output,
            &format!(
                "delete_folder: {} -> {}",
                path.display(),
                resolved.display()
            ),
        );
        Ok(self.outcome(
            OpKind::Trash,
            format!("Deleted {name} to trash"),
            path,
            &resolved,
            false,
            None,
        ))
    }

    /// Recursively fuzzy-search every folder under the output root (skipping the
    /// `.comfysort` dir). Returns the top matches sorted by score desc then name
    /// asc, capped at 50. An empty query returns an empty vec.
    pub fn search_folders(&self, query: &str) -> Vec<FolderEntry> {
        if query.trim().is_empty() {
            return Vec::new();
        }
        let mut scored = search::walk(&self.output, STATE_DIR, query);
        scored.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| a.rel.cmp(&b.rel))
        });
        scored.truncate(50);
        scored
            .into_iter()
            .map(|s| {
                let name = Path::new(&s.path)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| s.rel.clone());
                FolderEntry {
                    media_count: count_media(&s.path),
                    subfolder_count: count_subfolders(&s.path),
                    path: s.path.to_string_lossy().into_owned(),
                    name,
                }
            })
            .collect()
    }

    /// Create a new folder under `parent` and return it as a destination.
    /// Refreshes the live destination list so a fresh hotkey can be assigned.
    pub fn create_folder(&mut self, parent: &Path, name: &str) -> anyhow::Result<DestinationDto> {
        let clean = name.trim();
        if clean.is_empty() || clean.contains(['/', '\\']) {
            anyhow::bail!("invalid folder name");
        }
        let path = parent.join(clean);
        std::fs::create_dir_all(&path)?;
        self.destinations = scan_destinations(&self.output)?;
        let created = self
            .destinations
            .iter()
            .find(|d| Path::new(&d.path) == path)
            .cloned()
            .unwrap_or(DestinationDto {
                label: clean.to_owned(),
                path: path.to_string_lossy().into_owned(),
                hotkey: None,
                is_trash: false,
                media_count: 0,
            });
        Ok(created)
    }

    /// Re-count media in every destination and return the refreshed list.
    fn refreshed_destinations(&mut self) -> Vec<DestinationDto> {
        for dest in &mut self.destinations {
            dest.media_count = crate::destinations::count_media(Path::new(&dest.path));
        }
        self.destinations.clone()
    }

    fn label_for_dir(&self, dir: &Path) -> String {
        self.destinations
            .iter()
            .find(|d| Path::new(&d.path) == dir)
            .map(|d| d.label.clone())
            .unwrap_or_else(|| {
                dir.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| dir.to_string_lossy().into_owned())
            })
    }

    #[allow(clippy::too_many_arguments)]
    fn outcome(
        &mut self,
        kind: OpKind,
        message: String,
        source: &Path,
        resolved: &Path,
        source_removed: bool,
        restored_item: Option<MediaItemDto>,
    ) -> OpOutcome {
        OpOutcome {
            message,
            kind,
            source_path: source.to_string_lossy().into_owned(),
            resolved_path: resolved.to_string_lossy().into_owned(),
            source_removed,
            restored_item,
            can_undo: self.engine.can_undo(),
            destinations: self.refreshed_destinations(),
        }
    }
}

/// Count immediate child directories (excluding the reserved state dir).
fn count_subfolders(path: &Path) -> usize {
    std::fs::read_dir(path)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| {
                    e.metadata().map(|m| m.is_dir()).unwrap_or(false)
                        && !e
                            .file_name()
                            .to_string_lossy()
                            .eq_ignore_ascii_case(STATE_DIR)
                })
                .count()
        })
        .unwrap_or(0)
}
