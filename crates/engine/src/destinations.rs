//! Destination scanning: child folders of the output root become sort targets.
//!
//! Unlike the TUI (which deliberately never auto-grabs digit slots), the GUI's
//! milestone-1 loop auto-assigns hotkeys `1`–`9` to the first nine non-trash
//! destinations so the keyboard loop works immediately. Clicking a target works
//! regardless of hotkey. Trash always binds to `0`.

use crate::domain::{DestinationDto, MediaKind, STATE_DIR, media_kind};
use std::path::Path;

const HOTKEY_SLOTS: &[&str] = &["1", "2", "3", "4", "5", "6", "7", "8", "9"];

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
        if !entry.metadata().map(|m| m.is_dir()).unwrap_or(false) {
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
    for (slot, dest) in HOTKEY_SLOTS.iter().zip(normal.iter_mut()) {
        dest.hotkey = Some((*slot).to_owned());
    }

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
                .filter(|e| {
                    e.metadata().map(|m| m.is_file()).unwrap_or(false)
                        && media_kind(&e.path()) != MediaKind::Other
                })
                .count()
        })
        .unwrap_or(0)
}
