//! Destination scanning: child folders of the output root become sort targets.
//!
//! Like the TUI, the scanner deliberately never auto-grabs digit slots: scanned
//! folders get no hotkey. Hotkeys come only from user binds (persisted in
//! `bindings.json` and applied by the session on open). Clicking a target works
//! regardless of hotkey. Trash always binds to `0`.

use crate::domain::{DestinationDto, MediaKind, STATE_DIR, media_kind};
use std::path::Path;

/// Scan the output root's immediate child directories as destinations.
/// The reserved `.comfysort` state dir is excluded. A `.trash`/`trash` folder
/// (if present at the top level) is surfaced and bound to `0`.
pub fn scan_destinations(output_root: &Path) -> std::io::Result<Vec<DestinationDto>> {
    let mut normal = Vec::new();
    let mut trash = None;

    for entry in std::fs::read_dir(output_root)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        // `file_type()` is served from the directory enumeration and is cheaper
        // than a full `metadata()` when we only need the dir/file bit.
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let label = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if label.eq_ignore_ascii_case(STATE_DIR) {
            continue;
        }
        let is_trash = label == ".trash" || label.eq_ignore_ascii_case("trash");
        let dto = DestinationDto {
            media_count: count_media(&path),
            label,
            path: path.to_string_lossy().into_owned(),
            hotkey: None,
            is_trash,
        };
        if is_trash {
            trash = Some(dto);
        } else {
            normal.push(dto);
        }
    }

    normal.sort_by(|a, b| a.label.cmp(&b.label));

    let mut out = normal;
    let mut trash = trash.unwrap_or_else(|| synthetic_trash(output_root));
    trash.hotkey = Some("0".to_owned());
    out.push(trash);
    Ok(out)
}

/// The trash target is virtual: `<output>/.comfysort/.trash`. It may not exist
/// on disk yet; it's created lazily on first trash operation.
fn synthetic_trash(output_root: &Path) -> DestinationDto {
    let path = crate::domain::trash_dir(output_root);
    DestinationDto {
        media_count: count_media(&path),
        label: "trash".to_owned(),
        path: path.to_string_lossy().into_owned(),
        hotkey: Some("0".to_owned()),
        is_trash: true,
    }
}

/// Count immediate-child media files in a folder (0 if unreadable/missing).
pub fn count_media(path: &Path) -> usize {
    std::fs::read_dir(path)
        .map(|entries| {
            entries
                .flatten()
                // `file_type()` (free from the read_dir pass) instead of a full
                // `metadata()` stat; `media_kind` is path-only. One pass, no
                // per-entry re-stat via absolute paths.
                .filter(|e| {
                    e.file_type().map(|t| t.is_file()).unwrap_or(false)
                        && media_kind(&e.path()) != MediaKind::Other
                })
                .count()
        })
        .unwrap_or(0)
}

/// Recursively count media files (`media_kind != Other`) in a folder's whole
/// subtree (0 if unreadable/missing). Used by the Navigator so a folder holding
/// only subfolders still shows its true descendant total instead of `(0)`.
///
/// This walks the *entire* subtree (one `read_dir` per directory, `file_type()`
/// for the dir/file bit — no extra stat). The reserved `.comfysort` state dir is
/// skipped. Symlinked directories are never recursed into: `file_type().is_dir()`
/// is `false` for a symlink, so the walk only descends into real directories,
/// which both avoids following links and rules out symlink-based cycles. The walk
/// is iterative (explicit stack), so a very deep tree can't blow the call stack.
///
/// Unlike [`count_media`] and the per-op destination bumps (which stay cheap and
/// immediate), this cost is borne only on navigation — an on-demand, frontend-
/// debounced action — so the deeper walk is acceptable there.
pub fn count_media_recursive(path: &Path) -> usize {
    let mut total = 0usize;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            // `file_type()` is served from the directory enumeration — cheaper
            // than a full `metadata()` and, crucially, reports a symlink as
            // neither file nor (followed) dir, so links are skipped entirely.
            let ft = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if ft.is_dir() {
                if entry
                    .file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(STATE_DIR)
                {
                    continue;
                }
                stack.push(entry.path());
            } else if ft.is_file() && media_kind(&entry.path()) != MediaKind::Other {
                total += 1;
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn count_media_recursive_sums_subtree_and_skips_state_dir() {
        let dir = tempdir().unwrap();
        let parent = dir.path().join("parent");
        let a = parent.join("a");
        let b = parent.join("b");
        let state = parent.join(STATE_DIR);
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        fs::create_dir_all(&state).unwrap();

        // `parent` itself holds no direct media file — only a non-media note and
        // subfolders — so the immediate count is 0 but the subtree total is 2.
        fs::write(parent.join("note.txt"), b"x").unwrap();
        fs::write(a.join("1.jpg"), b"img").unwrap();
        fs::write(b.join("2.png"), b"img").unwrap();
        // Media under the reserved state dir must NOT be counted.
        fs::write(state.join("hidden.jpg"), b"img").unwrap();

        assert_eq!(count_media(&parent), 0, "no immediate media in parent");
        assert_eq!(
            count_media_recursive(&parent),
            2,
            "recursive total counts a/1.jpg and b/2.png, skipping note.txt and .comfysort"
        );
    }

    #[test]
    fn count_media_recursive_missing_dir_is_zero() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("nope");
        assert_eq!(count_media_recursive(&missing), 0);
    }
}
