//! Inbox scanning: enumeration of supported media in the input root — flat by
//! default, or walking the whole subtree when `recursive` is set.

use crate::domain::{MediaItemDto, MediaKind, STATE_DIR, media_kind, system_time_to_ms_pub};
use std::path::Path;

/// Scan the input directory for supported media, newest first. With
/// `recursive`, every subfolder is walked too (the `.comfysort` state dir is
/// skipped, and symlinks/junctions are never followed, so the walk can't cycle).
pub fn scan_inbox(root: &Path, recursive: bool) -> std::io::Result<Vec<MediaItemDto>> {
    let mut items = Vec::new();
    collect(root, recursive, &mut items)?;
    // Newest first is the most useful default for triage; fall back to name.
    items.sort_by(|a, b| {
        b.modified_ms
            .cmp(&a.modified_ms)
            .then_with(|| a.file_name.cmp(&b.file_name))
    });
    Ok(items)
}

/// Gather media in `dir` into `items`. The top-level read_dir error propagates
/// (a missing/unreadable root is a real failure); an unreadable *subfolder*
/// during a recursive walk is skipped so one bad directory can't sink the scan.
fn collect(dir: &Path, recursive: bool, items: &mut Vec<MediaItemDto>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        // One metadata call per entry, served from the directory enumeration on
        // Windows (`entry.metadata()`, not `fs::metadata(path)` which re-stats).
        // It does not follow symlinks, so a junction/symlinked dir reports as a
        // symlink (neither file nor dir) and falls through both branches.
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.is_dir() {
            if !recursive {
                continue;
            }
            let name = entry.file_name();
            if name.to_string_lossy().eq_ignore_ascii_case(STATE_DIR) {
                continue;
            }
            let _ = collect(&path, true, items);
            continue;
        }
        if !meta.is_file() {
            continue;
        }
        let kind = media_kind(&path);
        if kind == MediaKind::Other {
            continue;
        }
        items.push(MediaItemDto {
            file_name: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            kind,
            size_bytes: meta.len(),
            modified_ms: meta.modified().ok().and_then(system_time_to_ms_pub),
            path: path.to_string_lossy().into_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn names(items: &[MediaItemDto]) -> Vec<&str> {
        let mut v: Vec<&str> = items.iter().map(|i| i.file_name.as_str()).collect();
        v.sort_unstable();
        v
    }

    #[test]
    fn flat_scan_ignores_subfolders() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("top.jpg"), b"x").unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("nested.jpg"), b"x").unwrap();

        let items = scan_inbox(dir.path(), false).unwrap();
        assert_eq!(names(&items), ["top.jpg"]);
    }

    #[test]
    fn recursive_scan_walks_subfolders() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("top.jpg"), b"x").unwrap();
        let deep = dir.path().join("a").join("b");
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("nested.mp4"), b"x").unwrap();
        fs::write(deep.join("not-media.txt"), b"x").unwrap();

        let items = scan_inbox(dir.path(), true).unwrap();
        assert_eq!(names(&items), ["nested.mp4", "top.jpg"]);
    }

    #[test]
    fn recursive_scan_skips_state_dir() {
        let dir = tempdir().unwrap();
        let state = dir.path().join(STATE_DIR).join(".trash");
        fs::create_dir_all(&state).unwrap();
        fs::write(state.join("trashed.jpg"), b"x").unwrap();
        fs::write(dir.path().join("top.jpg"), b"x").unwrap();

        let items = scan_inbox(dir.path(), true).unwrap();
        assert_eq!(names(&items), ["top.jpg"]);
    }
}
