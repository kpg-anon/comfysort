//! In-memory session state: the roots, the live destination list, and the
//! operation engine. Wrapped in a `Mutex` and `manage`d by Tauri.

use crate::destinations::{count_media, scan_destinations};
use crate::domain::{
    DestinationDto, FolderEntry, FolderListing, MediaItemDto, OpKind, OpOutcome, STATE_DIR,
    SessionView, journal_path, trash_dir,
};
use crate::media::scan_inbox;
use crate::operations::{CompletedOp, OperationEngine, OperationKind};
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
            OperationKind::UndoMove | OperationKind::UndoCopy => {
                anyhow::bail!("undo records are not themselves undoable")
            }
        }
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
