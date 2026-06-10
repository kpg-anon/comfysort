//! In-memory session state: the roots, the live destination list, and the
//! operation engine. Wrapped in a `Mutex` and `manage`d by Tauri.

use crate::destinations::{count_media, count_media_recursive, scan_destinations};
use crate::domain::{
    CollisionPolicy, DestinationDto, FolderEntry, FolderListing, MediaItemDto, OpKind, OpOutcome,
    STATE_DIR, SessionView, journal_path, trash_dir,
};
use crate::logging::log;
use crate::media::scan_inbox;
use crate::operations::{CompletedOp, OperationEngine, OperationKind};
use crate::persistence::PersistedBindings;
use crate::search;
use std::path::{Path, PathBuf};

/// Hotkey characters the bind flow accepts. Trash (`'0'`) is reserved and
/// auto-bound by the scanner, never reassigned through this flow.
fn is_bindable_hotkey(hotkey: char) -> bool {
    matches!(hotkey, '1'..='9' | '-' | '=')
}

/// Scan one or more `;`-separated input directories, merging their media into a
/// single newest-first inbox. Empty segments are ignored. With `recursive`,
/// every input's subtree is walked instead of just its top level.
fn scan_inputs(input: &str, recursive: bool) -> anyhow::Result<Vec<MediaItemDto>> {
    let mut items = Vec::new();
    for dir in input.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        items.extend(scan_inbox(Path::new(dir), recursive)?);
    }
    items.sort_by(|a, b| {
        b.modified_ms
            .cmp(&a.modified_ms)
            .then_with(|| a.file_name.cmp(&b.file_name))
    });
    Ok(items)
}

pub struct Session {
    /// One or more `;`-separated input directories to triage.
    input: String,
    output: PathBuf,
    destinations: Vec<DestinationDto>,
    engine: OperationEngine,
    user_bindings: PersistedBindings,
    /// Collision policy applied to user-initiated moves/copies. Trash and folder
    /// delete always force `Rename` regardless of this, so they never clobber.
    collision_policy: CollisionPolicy,
    /// Walk inbox subfolders too (Settings "Recursive inbox scan").
    recursive_inbox: bool,
}

impl Session {
    /// Open a session against the given roots, scanning inbox + destinations.
    /// `recursive` walks every inbox subfolder instead of just the top level.
    pub fn open(
        input: String,
        output: PathBuf,
        recursive: bool,
    ) -> anyhow::Result<(Self, SessionView)> {
        let inbox = scan_inputs(&input, recursive)?;
        let mut destinations = scan_destinations(&output)?;
        let engine = OperationEngine::new(journal_path(&output));

        // Restore user-bound hotkeys persisted from prior sessions. Applied on
        // top of the scanned destinations: a bound top-level folder just gets
        // its hotkey set; a bound *nested* folder (not a top-level child) is
        // pushed as a new destination so it appears in the list. Matches the
        // TUI's `apply_user_bindings`.
        let user_bindings = PersistedBindings::load(&output);
        for (hotkey, abs_path) in user_bindings.resolved(&output) {
            if !is_bindable_hotkey(hotkey) {
                continue;
            }
            for dest in destinations.iter_mut() {
                if dest.hotkey.as_deref() == Some(&hotkey.to_string()) {
                    dest.hotkey = None;
                }
            }
            if let Some(existing) = destinations
                .iter_mut()
                .find(|d| Path::new(&d.path) == abs_path)
            {
                existing.hotkey = Some(hotkey.to_string());
                log(
                    &output,
                    &format!("bind restore: [{hotkey}] -> {}", abs_path.display()),
                );
                continue;
            }
            let label = abs_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "(custom)".to_owned());
            destinations.push(DestinationDto {
                media_count: count_media(&abs_path),
                label,
                path: abs_path.to_string_lossy().into_owned(),
                hotkey: Some(hotkey.to_string()),
                is_trash: false,
            });
            log(
                &output,
                &format!("bind restore (new): [{hotkey}] -> {}", abs_path.display()),
            );
        }

        // Default the "=" slot to a managed archive folder under the state dir,
        // unless the user has already bound "=" to something else. It lives in
        // `.comfysort/archive` so it never shows up as a scanned destination.
        if !destinations.iter().any(|d| d.hotkey.as_deref() == Some("=")) {
            let archive = output.join(STATE_DIR).join("archive");
            let _ = std::fs::create_dir_all(&archive);
            destinations.push(DestinationDto {
                media_count: count_media(&archive),
                label: "archive".to_owned(),
                path: archive.to_string_lossy().into_owned(),
                hotkey: Some("=".to_owned()),
                is_trash: false,
            });
        }

        // Session-open diagnostic banner: exactly what the scanner found this
        // launch, so a reported file-disappearance can be traced to the scan.
        log(
            &output,
            &format!(
                "session open: input={} output={} destinations={} inbox={}",
                input,
                output.display(),
                destinations.len(),
                inbox.len()
            ),
        );
        for item in &inbox {
            log(&output, &format!("  inbox item: {}", item.file_name));
        }

        let view = SessionView {
            input: input.clone(),
            output: output.to_string_lossy().into_owned(),
            inbox,
            destinations: destinations.clone(),
        };
        let session = Self {
            input,
            output,
            destinations,
            engine,
            user_bindings,
            collision_policy: CollisionPolicy::Rename,
            recursive_inbox: recursive,
        };
        Ok((session, view))
    }

    /// Set the collision policy used by subsequent user moves/copies. Trash and
    /// folder delete are unaffected (they always rename to avoid clobbering).
    pub fn set_collision_policy(&mut self, p: CollisionPolicy) {
        self.collision_policy = p;
    }

    /// Toggle the recursive inbox walk for subsequent rescans (the Settings
    /// toggle flips this on the live session, then triggers a refresh).
    pub fn set_recursive_inbox(&mut self, recursive: bool) {
        self.recursive_inbox = recursive;
    }

    /// Re-scan the input directory (e.g. after external changes) and return the
    /// fresh inbox. Destinations are left as-is; call after a manual refresh.
    pub fn rescan_inbox(&self) -> anyhow::Result<Vec<MediaItemDto>> {
        scan_inputs(&self.input, self.recursive_inbox)
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
                // `file_type()` (from the enumeration) over a full `metadata()`
                // stat â€” we only need the dir bit here.
                if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
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
                    // Recursive subtree total so a parent holding only subfolders
                    // still shows its true descendant media count instead of (0).
                    // `subfolder_count` stays immediate â€” it only drives a
                    // "has children" indicator. The deeper walk's cost is borne on
                    // navigation (on-demand, debounced on the frontend).
                    media_count: count_media_recursive(&path),
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
        let resolved = self
            .engine
            .move_file(source, dest_dir, self.collision_policy)?;
        log(
            &self.output,
            &format!("move: {} -> {}", source.display(), resolved.display()),
        );
        // Incremental count: the file landed in `dest_dir`; bump only that
        // destination instead of re-reading every destination directory.
        self.bump_dest_count(dest_dir, 1);
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
        let resolved = self
            .engine
            .copy_file(source, dest_dir, self.collision_policy)?;
        log(
            &self.output,
            &format!("copy: {} -> {}", source.display(), resolved.display()),
        );
        // Incremental count: a duplicate now lives in `dest_dir`.
        self.bump_dest_count(dest_dir, 1);
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
        // Trash must never clobber, regardless of the user's collision setting.
        let resolved = self
            .engine
            .move_file(source, &dir, CollisionPolicy::Rename)?;
        log(
            &self.output,
            &format!("trash: {} -> {}", source.display(), resolved.display()),
        );
        // Incremental count: the trash destination gained a file.
        self.bump_dest_count(&dir, 1);
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
                // A reversed move/trash restores the file to the inbox, so the
                // destination it left (the parent dir it sat in) loses one.
                if let Some(left) = resolved_path.parent() {
                    self.bump_dest_count(left, -1);
                }
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
            OperationKind::Copy => {
                // The duplicate was removed from its destination.
                if let Some(left) = resolved_path.parent() {
                    self.bump_dest_count(left, -1);
                }
                Ok(self.outcome(
                    OpKind::Undo,
                    "Undo: removed duplicate".to_owned(),
                    &resolved_path,
                    &resolved_path,
                    false,
                    None,
                ))
            }
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
        // Prune any persisted bindings pointing at the deleted folder (or
        // anything nested under it) so a hotkey doesn't dangle next launch.
        self.user_bindings.remove_under(path, &self.output);
        if let Err(err) = self.user_bindings.save(&self.output) {
            log(
                &self.output,
                &format!("persist bindings failed during delete: {err}"),
            );
        }
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

    /// Create a new folder under `parent` and return it as a destination DTO.
    /// Does NOT touch `self.destinations`: a new folder isn't a sort target until
    /// the user binds it, and re-scanning here would drop the applied hotkey
    /// bindings (they're only re-applied on session open).
    pub fn create_folder(&mut self, parent: &Path, name: &str) -> anyhow::Result<DestinationDto> {
        let clean = name.trim();
        if clean.is_empty() || clean.contains(['/', '\\']) {
            anyhow::bail!("invalid folder name");
        }
        let path = parent.join(clean);
        std::fs::create_dir_all(&path)?;
        Ok(DestinationDto {
            label: clean.to_owned(),
            path: path.to_string_lossy().into_owned(),
            hotkey: None,
            is_trash: false,
            media_count: 0,
        })
    }

    /// Bind a folder under the output subtree to a hotkey (`1..=9`, `-`, `=`).
    /// Enforces hotkey uniqueness (strips it from any prior holder), sets it on
    /// the matching destination or pushes a new one for a nested folder, and
    /// persists the binding. Returns the refreshed destination list.
    pub fn bind_folder(&mut self, path: &Path, hotkey: char) -> anyhow::Result<Vec<DestinationDto>> {
        let path = self.clamp_to_output(path);
        self.bind_resolved(path, hotkey)
    }

    /// Bind an absolute path that may live *outside* the output root (used by the
    /// Settings sort-target editor, where the user can target any folder on disk).
    pub fn bind_path(&mut self, path: &Path, hotkey: char) -> anyhow::Result<Vec<DestinationDto>> {
        self.bind_resolved(path.to_path_buf(), hotkey)
    }

    fn bind_resolved(&mut self, path: PathBuf, hotkey: char) -> anyhow::Result<Vec<DestinationDto>> {
        if !is_bindable_hotkey(hotkey) {
            anyhow::bail!("bind hotkey must be 1-9, -, or =");
        }
        let key = hotkey.to_string();

        // Strip the hotkey from any destination currently holding it so the
        // slot is unique. Trash ('0') is never reached here (not bindable).
        for dest in self.destinations.iter_mut() {
            if dest.hotkey.as_deref() == Some(&key) {
                dest.hotkey = None;
            }
        }
        if let Some(existing) = self
            .destinations
            .iter_mut()
            .find(|d| Path::new(&d.path) == path.as_path())
        {
            existing.hotkey = Some(key);
        } else {
            let label = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "(custom)".to_owned());
            self.destinations.push(DestinationDto {
                media_count: count_media(&path),
                label,
                path: path.to_string_lossy().into_owned(),
                hotkey: Some(key),
                is_trash: false,
            });
        }

        self.user_bindings.set(hotkey, &path, &self.output);
        self.user_bindings.save(&self.output)?;
        log(
            &self.output,
            &format!("bind: [{hotkey}] -> {}", path.display()),
        );
        Ok(self.refreshed_destinations())
    }

    /// Rename a folder under the output tree in place. Refuses the root, the
    /// state dir, and the trash dir; refuses if the target name already exists.
    /// Updates any in-memory destination + persisted binding pointing at the old
    /// path, then returns the refreshed listing of the parent directory.
    pub fn rename_folder(&mut self, path: &Path, new_name: &str) -> anyhow::Result<FolderListing> {
        let clean = new_name.trim();
        if clean.is_empty() || clean.contains(['/', '\\']) {
            anyhow::bail!("invalid folder name");
        }
        let state = self.output.join(STATE_DIR);
        let trash = trash_dir(&self.output);
        if path == self.output {
            anyhow::bail!("refusing to rename the output root");
        }
        if path == state || path.starts_with(&state) {
            anyhow::bail!("refusing to rename the .comfysort state directory");
        }
        if path == trash {
            anyhow::bail!("refusing to rename the trash directory");
        }
        let parent = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("folder has no parent"))?
            .to_path_buf();
        let target = parent.join(clean);
        if target.exists() {
            anyhow::bail!("a folder named \"{clean}\" already exists here");
        }
        std::fs::rename(path, &target)?;
        for dest in self.destinations.iter_mut() {
            if Path::new(&dest.path) == path {
                dest.path = target.to_string_lossy().into_owned();
                dest.label = clean.to_owned();
            }
        }
        self.user_bindings.rename_under(path, &target, &self.output);
        if let Err(err) = self.user_bindings.save(&self.output) {
            log(&self.output, &format!("persist bindings failed during rename: {err}"));
        }
        log(
            &self.output,
            &format!("rename: {} -> {}", path.display(), target.display()),
        );
        self.list_folders(&parent)
    }

    /// Revert one specific past operation (per-file undo from the history view).
    /// A reverted move/trash restores the file to the inbox; a reverted copy
    /// removes the duplicate. `source`/`resolved` identify the recorded op.
    pub fn revert_op(&mut self, source: &Path, resolved: &Path) -> anyhow::Result<OpOutcome> {
        let kind = self.engine.revert_specific(source, resolved)?;
        // The folder the file left loses one (mirrors undo's count handling).
        if let Some(left) = resolved.parent() {
            self.bump_dest_count(left, -1);
        }
        log(
            &self.output,
            &format!("revert {:?}: {} -> {}", kind, resolved.display(), source.display()),
        );
        match kind {
            OperationKind::Copy => Ok(self.outcome(
                OpKind::Undo,
                "Reverted â€” removed copy".to_owned(),
                resolved,
                resolved,
                false,
                None,
            )),
            _ => {
                let restored = MediaItemDto::from_path(source);
                Ok(self.outcome(
                    OpKind::Undo,
                    "Reverted â€” restored to inbox".to_owned(),
                    resolved,
                    source,
                    false,
                    restored,
                ))
            }
        }
    }

    /// Clear a hotkey binding. A scanned top-level folder just loses its hotkey;
    /// a folder that was only present because of a bind to a non-scanned (nested)
    /// path is dropped from the list entirely. Persists the removal.
    pub fn unbind_hotkey(&mut self, hotkey: char) -> anyhow::Result<Vec<DestinationDto>> {
        let key = hotkey.to_string();
        let top_level = self.scanned_top_level();
        let is_top_level = |path: &str| top_level.iter().any(|p| p == Path::new(path));

        self.destinations.retain_mut(|dest| {
            if dest.hotkey.as_deref() != Some(&key) {
                return true;
            }
            if dest.is_trash || is_top_level(&dest.path) {
                // Real destination â€” keep it, just drop the hotkey.
                dest.hotkey = None;
                true
            } else {
                // Only present because of the bind â€” remove it entirely.
                false
            }
        });

        self.user_bindings.remove_hotkey(hotkey);
        self.user_bindings.save(&self.output)?;
        log(&self.output, &format!("unbind: [{hotkey}]"));
        Ok(self.refreshed_destinations())
    }

    /// The immediate child directories of the output root (scanned top-level
    /// folders), excluding the reserved state dir. Used to decide whether an
    /// unbound destination is a real folder or only existed because of a bind.
    fn scanned_top_level(&self) -> Vec<PathBuf> {
        let mut out = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.output) {
            for entry in entries.flatten() {
                let path = entry.path();
                // `file_type()` from the enumeration; only the dir bit is needed.
                if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    continue;
                }
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if name.eq_ignore_ascii_case(STATE_DIR) {
                    continue;
                }
                out.push(path);
            }
        }
        out
    }

    /// Re-count media in every destination and return the refreshed list.
    /// Kept for bind/unbind, which restructure the destination list; the hot
    /// per-op path uses [`Self::bump_dest_count`] instead to avoid N read_dirs.
    fn refreshed_destinations(&mut self) -> Vec<DestinationDto> {
        for dest in &mut self.destinations {
            dest.media_count = crate::destinations::count_media(Path::new(&dest.path));
        }
        self.destinations.clone()
    }

    /// Adjust the in-memory `media_count` of the destination whose path equals
    /// `dir` by `delta` (saturating at 0). Matched by `Path` equality, not
    /// string. If no destination row matches (e.g. a move into a deep nested
    /// folder that isn't a bound destination) this is a no-op â€” there's simply
    /// nothing to bump. This replaces a full destination rescan per operation:
    /// O(num_destinations) `read_dir` calls become 0.
    fn bump_dest_count(&mut self, dir: &Path, delta: i64) {
        if let Some(dest) = self
            .destinations
            .iter_mut()
            .find(|d| Path::new(&d.path) == dir)
        {
            if delta >= 0 {
                dest.media_count += delta as usize;
            } else {
                dest.media_count = dest.media_count.saturating_sub((-delta) as usize);
            }
        }
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
            // Counts were already adjusted incrementally by the calling op
            // method (move/copy/trash/undo). No full rescan here: a single
            // operation touches the filesystem only for the file move/copy
            // plus its journal, never for an N-destination recount.
            destinations: self.destinations.clone(),
        }
    }
}

/// Count immediate child directories (excluding the reserved state dir).
fn count_subfolders(path: &Path) -> usize {
    std::fs::read_dir(path)
        .map(|entries| {
            entries
                .flatten()
                // `file_type()` (free from read_dir) over a full `metadata()`
                // stat â€” single pass, only the dir bit is needed.
                .filter(|e| {
                    e.file_type().map(|t| t.is_dir()).unwrap_or(false)
                        && !e
                            .file_name()
                            .to_string_lossy()
                            .eq_ignore_ascii_case(STATE_DIR)
                })
                .count()
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// Find a destination's in-memory media count by absolute path.
    fn count_of(session: &Session, dir: &Path) -> usize {
        session
            .destinations
            .iter()
            .find(|d| Path::new(&d.path) == dir)
            .map(|d| d.media_count)
            .expect("destination present")
    }

    #[test]
    fn move_increments_only_target_count_without_rescanning_others() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("inbox");
        let output = dir.path().join("out");
        let keep = output.join("keep");
        let other = output.join("other");
        fs::create_dir_all(&input).unwrap();
        fs::create_dir_all(&keep).unwrap();
        fs::create_dir_all(&other).unwrap();
        let src = input.join("a.jpg");
        fs::write(&src, b"img").unwrap();

        let (mut session, _view) = Session::open(input.to_string_lossy().into_owned(), output.clone(), false).unwrap();
        assert_eq!(count_of(&session, &keep), 0);

        // Poison `other`'s in-memory count with a sentinel. If the op path did a
        // full rescan, this would be recomputed back to 0; it must survive,
        // proving the op only touched the target destination.
        for d in session.destinations.iter_mut() {
            if Path::new(&d.path) == other {
                d.media_count = 999;
            }
        }

        let outcome = session.move_item(&src, &keep).unwrap();

        // Target bumped by exactly 1.
        assert_eq!(count_of(&session, &keep), 1);
        // Other destination untouched (no rescan occurred).
        assert_eq!(count_of(&session, &other), 999);
        // The returned DTOs carry the same incrementally-updated counts.
        let dto_keep = outcome
            .destinations
            .iter()
            .find(|d| Path::new(&d.path) == keep)
            .unwrap();
        assert_eq!(dto_keep.media_count, 1);
        let dto_other = outcome
            .destinations
            .iter()
            .find(|d| Path::new(&d.path) == other)
            .unwrap();
        assert_eq!(dto_other.media_count, 999);
    }

    #[test]
    fn undo_move_decrements_target_count() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("inbox");
        let output = dir.path().join("out");
        let keep = output.join("keep");
        fs::create_dir_all(&input).unwrap();
        fs::create_dir_all(&keep).unwrap();
        let src = input.join("a.jpg");
        fs::write(&src, b"img").unwrap();

        let (mut session, _view) = Session::open(input.to_string_lossy().into_owned(), output.clone(), false).unwrap();
        session.move_item(&src, &keep).unwrap();
        assert_eq!(count_of(&session, &keep), 1);

        session.undo().unwrap();
        assert_eq!(count_of(&session, &keep), 0, "undo restores the count");
    }

    #[test]
    fn copy_increments_and_undo_decrements() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("inbox");
        let output = dir.path().join("out");
        let keep = output.join("keep");
        fs::create_dir_all(&input).unwrap();
        fs::create_dir_all(&keep).unwrap();
        let src = input.join("a.jpg");
        fs::write(&src, b"img").unwrap();

        let (mut session, _view) = Session::open(input.to_string_lossy().into_owned(), output.clone(), false).unwrap();
        session.copy_item(&src, &keep).unwrap();
        assert_eq!(count_of(&session, &keep), 1);

        session.undo().unwrap();
        assert_eq!(count_of(&session, &keep), 0);
    }

    #[test]
    fn trash_increments_trash_count() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("inbox");
        let output = dir.path().join("out");
        fs::create_dir_all(&input).unwrap();
        fs::create_dir_all(&output).unwrap();
        let src = input.join("a.jpg");
        fs::write(&src, b"img").unwrap();

        let (mut session, _view) = Session::open(input.to_string_lossy().into_owned(), output.clone(), false).unwrap();
        let trash = trash_dir(&output);
        assert_eq!(count_of(&session, &trash), 0);

        session.trash_item(&src).unwrap();
        assert_eq!(count_of(&session, &trash), 1);
    }
}
