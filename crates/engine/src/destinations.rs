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
