//! Inbox scanning: flat enumeration of supported media in the input root.

use crate::domain::{MediaItemDto, MediaKind, media_kind, system_time_to_ms_pub};
use std::path::Path;

/// Flat-scan the input directory for supported media, newest first.
pub fn scan_inbox(root: &Path) -> std::io::Result<Vec<MediaItemDto>> {
    let mut items = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        let meta = match entry.metadata() {
            Ok(m) if m.is_file() => m,
            _ => continue,
        };
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
    // Newest first is the most useful default for triage; fall back to name.
    items.sort_by(|a, b| {
        b.modified_ms
            .cmp(&a.modified_ms)
            .then_with(|| a.file_name.cmp(&b.file_name))
    });
    Ok(items)
}
