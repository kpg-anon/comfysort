//! In-memory session state: the roots, the live destination list, and the
//! operation engine. Wrapped in a `Mutex` and `manage`d by Tauri.

use crate::destinations::{count_media, count_media_recursive, scan_destinations};
use crate::domain::{
    CollisionPolicy, DestinationDto, EmptyTrashResult, FolderEntry, FolderListing, MediaItemDto,
    OpKind, OpOutcome, RenameResult, STATE_DIR, SessionView, journal_path, trash_dir,
};
use crate::ignore::IgnoreSet;
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
    /// Folders hidden from the Navigator, folder search, the sort-target scan,
    /// and recursive media counts (Settings "Ignored folders"). Applies to the
    /// output tree only — the inbox scan is a different root and is unaffected.
    ignores: IgnoreSet,
}

impl Session {
    /// Open a session against the given roots, scanning inbox + destinations.
    /// `recursive` walks every inbox subfolder instead of just the top level.
    /// `ignored` is the raw `ignoredFolders` config list (see [`IgnoreSet`]).
    pub fn open(
        input: String,
        output: PathBuf,
        recursive: bool,
        ignored: &[String],
    ) -> anyhow::Result<(Self, SessionView)> {
        let ignores = IgnoreSet::new(ignored);
        let inbox = scan_inputs(&input, recursive)?;
        let mut destinations = scan_destinations(&output, &ignores)?;
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
            ignores,
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

    /// Replace the ignore rules on the live session (the Navigator's "Ignore
    /// this folder" and the Settings list both land here) and return the
    /// refreshed destination list.
    ///
    /// Top-level destinations are re-scanned under the new rules so un-ignoring
    /// a folder brings it straight back. A bare re-scan would drop what the
    /// scanner never produces, so two things are carried over from the previous
    /// list: hotkeys (only re-applied from `bindings.json` on open), and bound
    /// folders that aren't top-level children of the output root (nested binds
    /// and the managed `=` archive). A hotkey bind is an explicit choice, so a
    /// bound folder stays a sort target even once it is ignored — it just stops
    /// appearing in the Navigator.
    pub fn set_ignored_folders(&mut self, entries: &[String]) -> Vec<DestinationDto> {
        self.ignores = IgnoreSet::new(entries);
        let previous = std::mem::take(&mut self.destinations);
        let mut scanned = match scan_destinations(&self.output, &self.ignores) {
            Ok(scanned) => scanned,
            Err(err) => {
                // An unreadable output root: keep what we had rather than
                // blanking the user's sort targets.
                log(&self.output, &format!("set_ignored_folders rescan failed: {err}"));
                self.destinations = previous;
                return self.destinations.clone();
            }
        };
        for old in previous {
            match scanned.iter_mut().find(|d| Path::new(&d.path) == Path::new(&old.path)) {
                Some(existing) => {
                    if old.hotkey.is_some() {
                        existing.hotkey = old.hotkey;
                    }
                }
                None => {
                    if old.hotkey.is_some() && !old.is_trash {
                        scanned.push(old);
                    }
                }
            }
        }
        self.destinations = scanned;
        log(
            &self.output,
            &format!(
                "ignored folders: {} rule(s), {} destinations",
                entries.iter().filter(|e| !e.trim().is_empty()).count(),
                self.destinations.len()
            ),
        );
        self.destinations.clone()
    }

    /// Re-scan the input directory (e.g. after external changes) and return the
    /// fresh inbox. Destinations are left as-is; call after a manual refresh.
    pub fn rescan_inbox(&self) -> anyhow::Result<Vec<MediaItemDto>> {
        scan_inputs(&self.input, self.recursive_inbox)
    }

    /// List the immediate child folders of `dir` for the Navigator. `dir` is
    /// clamped to the output-root subtree so the Navigator can never escape it.
    /// Folders are sorted by media count desc, then name; `.comfysort` and every
    /// ignored folder are hidden.
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
                if self.ignores.is_ignored(&path) {
                    continue;
                }
                folders.push(FolderEntry {
                    // Recursive subtree total so a parent holding only subfolders
                    // still shows its true descendant media count instead of (0).
                    // `subfolder_count` stays immediate â€” it only drives a
                    // "has children" indicator. The deeper walk's cost is borne on
                    // navigation (on-demand, debounced on the frontend).
                    media_count: count_media_recursive(&path, &self.ignores),
                    subfolder_count: count_subfolders(&path, &self.ignores),
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
    /// `.comfysort` dir and every ignored folder). Returns the top matches sorted
    /// by score desc then name asc, capped at 50. An empty query returns an
    /// empty vec.
    pub fn search_folders(&self, query: &str) -> Vec<FolderEntry> {
        if query.trim().is_empty() {
            return Vec::new();
        }
        let mut scored = search::walk(&self.output, STATE_DIR, query, &self.ignores);
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
                    subfolder_count: count_subfolders(&s.path, &self.ignores),
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
    /// path, then returns the refreshed listing of the parent directory together
    /// with the destinations (so a renamed sort target's label updates live).
    pub fn rename_folder(&mut self, path: &Path, new_name: &str) -> anyhow::Result<RenameResult> {
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
        let listing = self.list_folders(&parent)?;
        Ok(RenameResult {
            listing,
            destinations: self.destinations.clone(),
        })
    }

    /// Permanently delete every entry in the session trash directory. Files and
    /// subfolders alike are removed; the trash destination's count is reset to
    /// zero. Returns how many top-level entries were removed plus the refreshed
    /// destination list. This is irreversible — the frontend confirms first.
    pub fn empty_trash(&mut self) -> anyhow::Result<EmptyTrashResult> {
        let dir = trash_dir(&self.output);
        let mut removed = 0usize;
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let result = if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    std::fs::remove_dir_all(&path)
                } else {
                    std::fs::remove_file(&path)
                };
                match result {
                    Ok(()) => removed += 1,
                    Err(err) => log(
                        &self.output,
                        &format!("empty_trash: failed to remove {}: {err}", path.display()),
                    ),
                }
            }
        }
        if let Some(dest) = self.destinations.iter_mut().find(|d| d.is_trash) {
            dest.media_count = 0;
        }
        log(
            &self.output,
            &format!("empty_trash: removed {removed} entries"),
        );
        Ok(EmptyTrashResult {
            removed,
            destinations: self.destinations.clone(),
        })
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
    /// folders), excluding the reserved state dir and any ignored folder. Used to
    /// decide whether an unbound destination is a real folder or only existed
    /// because of a bind — an ignored folder counts as the latter, so clearing its
    /// hotkey drops the row instead of leaving a target you can't navigate to.
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
                if name.eq_ignore_ascii_case(STATE_DIR) || self.ignores.is_ignored(&path) {
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

/// Count immediate child directories (excluding the reserved state dir and any
/// ignored folder, so the "has children" arrow matches what the Navigator lists).
fn count_subfolders(path: &Path, ignores: &IgnoreSet) -> usize {
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
                        && !ignores.is_ignored(&e.path())
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

        let (mut session, _view) = Session::open(input.to_string_lossy().into_owned(), output.clone(), false, &[]).unwrap();
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

        let (mut session, _view) = Session::open(input.to_string_lossy().into_owned(), output.clone(), false, &[]).unwrap();
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

        let (mut session, _view) = Session::open(input.to_string_lossy().into_owned(), output.clone(), false, &[]).unwrap();
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

        let (mut session, _view) = Session::open(input.to_string_lossy().into_owned(), output.clone(), false, &[]).unwrap();
        let trash = trash_dir(&output);
        assert_eq!(count_of(&session, &trash), 0);

        session.trash_item(&src).unwrap();
        assert_eq!(count_of(&session, &trash), 1);
    }

    #[test]
    fn empty_trash_clears_contents_and_resets_count() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("inbox");
        let output = dir.path().join("out");
        fs::create_dir_all(&input).unwrap();
        fs::create_dir_all(&output).unwrap();
        let a = input.join("a.jpg");
        let b = input.join("b.jpg");
        fs::write(&a, b"img").unwrap();
        fs::write(&b, b"img").unwrap();

        let (mut session, _view) = Session::open(input.to_string_lossy().into_owned(), output.clone(), false, &[]).unwrap();
        let trash = trash_dir(&output);
        session.trash_item(&a).unwrap();
        session.trash_item(&b).unwrap();
        assert_eq!(count_of(&session, &trash), 2);
        assert!(trash.exists());

        let result = session.empty_trash().unwrap();
        assert_eq!(result.removed, 2, "both trashed files removed");
        assert_eq!(count_of(&session, &trash), 0, "trash count reset");
        // The trash directory itself remains; only its contents are gone.
        let remaining = fs::read_dir(&trash).map(|e| e.count()).unwrap_or(0);
        assert_eq!(remaining, 0, "trash directory emptied");
    }

    // ---- Ignored folders ---------------------------------------------------

    /// An output root holding `keep/` (1 image) and `_raw/nested/` (2 images).
    fn ignore_fixture(dir: &Path) -> (PathBuf, PathBuf) {
        let input = dir.join("inbox");
        let output = dir.join("out");
        fs::create_dir_all(&input).unwrap();
        fs::create_dir_all(output.join("keep")).unwrap();
        fs::create_dir_all(output.join("_raw").join("nested")).unwrap();
        fs::write(output.join("keep").join("a.jpg"), b"img").unwrap();
        fs::write(output.join("_raw").join("b.jpg"), b"img").unwrap();
        fs::write(output.join("_raw").join("nested").join("c.jpg"), b"img").unwrap();
        (input, output)
    }

    fn open_with(input: &Path, output: &Path, ignored: &[&str]) -> Session {
        let ignored: Vec<String> = ignored.iter().map(|s| (*s).to_owned()).collect();
        Session::open(input.to_string_lossy().into_owned(), output.to_path_buf(), false, &ignored)
            .unwrap()
            .0
    }

    fn listed_names(session: &Session, dir: &Path) -> Vec<String> {
        session.list_folders(dir).unwrap().folders.into_iter().map(|f| f.name).collect()
    }

    fn total_listed_media(session: &Session, dir: &Path) -> usize {
        session.list_folders(dir).unwrap().folders.iter().map(|f| f.media_count).sum()
    }

    fn target_labels(session: &Session) -> Vec<String> {
        session.destinations.iter().map(|d| d.label.clone()).collect()
    }

    #[test]
    fn ignored_folder_is_hidden_from_navigator_search_and_targets() {
        let dir = tempdir().unwrap();
        let (input, output) = ignore_fixture(dir.path());
        let session = open_with(&input, &output, &["_raw"]);

        assert_eq!(listed_names(&session, &output), vec!["keep"]);
        let labels = target_labels(&session);
        assert!(!labels.contains(&"_raw".to_owned()), "{labels:?}");
        assert!(
            session.search_folders("raw").is_empty(),
            "fuzzy search must not surface an ignored folder"
        );
        // Its nested folder is unreachable too - the walk never descends past it.
        assert!(session.search_folders("nested").is_empty());
    }

    #[test]
    fn ignored_subtree_is_left_out_of_media_counts() {
        let dir = tempdir().unwrap();
        let (input, output) = ignore_fixture(dir.path());

        let visible = open_with(&input, &output, &[]);
        assert_eq!(
            total_listed_media(&visible, &output),
            3,
            "keep/a.jpg + _raw/b.jpg + _raw/nested/c.jpg"
        );

        let ignored = open_with(&input, &output, &["_raw"]);
        assert_eq!(
            total_listed_media(&ignored, &output),
            1,
            "only keep/a.jpg is still counted"
        );
    }

    #[test]
    fn absolute_path_rule_ignores_only_that_folder() {
        let dir = tempdir().unwrap();
        let (input, output) = ignore_fixture(dir.path());
        // A second `_raw` elsewhere in the tree must survive a path rule aimed at
        // the top-level one (a bare-name rule would take both).
        fs::create_dir_all(output.join("keep").join("_raw")).unwrap();
        let rule = output.join("_raw").to_string_lossy().into_owned();
        let session = open_with(&input, &output, &[&rule]);

        assert_eq!(listed_names(&session, &output), vec!["keep"]);
        assert_eq!(
            listed_names(&session, &output.join("keep")),
            vec!["_raw"],
            "the nested _raw is a different path and stays visible"
        );
    }

    #[test]
    fn setting_ignores_live_hides_then_restores_the_folder() {
        let dir = tempdir().unwrap();
        let (input, output) = ignore_fixture(dir.path());
        let mut session = open_with(&input, &output, &[]);
        assert!(target_labels(&session).contains(&"_raw".to_owned()));

        let after = session.set_ignored_folders(&["_raw".to_owned()]);
        assert!(!after.iter().any(|d| d.label == "_raw"));
        assert_eq!(listed_names(&session, &output), vec!["keep"]);

        // Un-ignoring re-scans, so the folder comes back as a sort target.
        let restored = session.set_ignored_folders(&[]);
        assert!(restored.iter().any(|d| d.label == "_raw"));
        assert!(listed_names(&session, &output).contains(&"_raw".to_owned()));
    }

    #[test]
    fn ignoring_keeps_a_bound_folder_as_a_target() {
        let dir = tempdir().unwrap();
        let (input, output) = ignore_fixture(dir.path());
        let mut session = open_with(&input, &output, &[]);
        session.bind_folder(&output.join("_raw"), '3').unwrap();

        let after = session.set_ignored_folders(&["_raw".to_owned()]);
        let bound = after
            .iter()
            .find(|d| d.hotkey.as_deref() == Some("3"))
            .expect("an explicit hotkey bind survives being ignored");
        assert_eq!(Path::new(&bound.path), output.join("_raw"));
        // ...but it is still gone from the Navigator.
        assert_eq!(listed_names(&session, &output), vec!["keep"]);
    }

    #[test]
    fn set_ignored_folders_preserves_the_trash_and_archive_slots() {
        let dir = tempdir().unwrap();
        let (input, output) = ignore_fixture(dir.path());
        let mut session = open_with(&input, &output, &[]);

        let after = session.set_ignored_folders(&["_raw".to_owned()]);
        assert!(
            after.iter().any(|d| d.is_trash && d.hotkey.as_deref() == Some("0")),
            "trash keeps slot 0"
        );
        assert!(
            after.iter().any(|d| d.hotkey.as_deref() == Some("=")),
            "the managed archive keeps slot ="
        );
    }
}
