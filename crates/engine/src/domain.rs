//! Core types and the serde DTOs sent across the IPC boundary.
//!
//! DTOs use `camelCase` so they land in the frontend as idiomatic TS objects.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Image,
    Video,
    Other,
}

/// Default collision policy: Windows-Explorer-style `name (2).ext` rename.
/// Never overwrite by default — a sort must never silently destroy a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionPolicy {
    Rename,
    Skip,
    Overwrite,
}

impl Default for CollisionPolicy {
    fn default() -> Self {
        Self::Rename
    }
}

/// A media file in the inbox.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaItemDto {
    pub path: String,
    pub file_name: String,
    pub kind: MediaKind,
    pub size_bytes: u64,
    /// Modified time in milliseconds since the Unix epoch, if available.
    pub modified_ms: Option<i64>,
}

impl MediaItemDto {
    /// Build a DTO from a path, statting it for size/mtime/kind.
    pub fn from_path(path: &Path) -> Option<Self> {
        let meta = std::fs::metadata(path).ok()?;
        if !meta.is_file() {
            return None;
        }
        let kind = media_kind(path);
        if kind == MediaKind::Other {
            return None;
        }
        Some(Self {
            file_name: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            kind,
            size_bytes: meta.len(),
            modified_ms: meta.modified().ok().and_then(system_time_to_ms),
            path: path.to_string_lossy().into_owned(),
        })
    }
}

/// A sort target (a child folder of the output root).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DestinationDto {
    pub label: String,
    pub path: String,
    /// The digit/char that triggers a move here, if assigned. `"0"` is trash.
    pub hotkey: Option<String>,
    pub is_trash: bool,
    pub media_count: usize,
}

/// A folder under the output root, as listed by the Navigator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderEntry {
    pub name: String,
    pub path: String,
    pub media_count: usize,
    pub subfolder_count: usize,
}

/// The contents of one directory in the Navigator tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderListing {
    /// Absolute path of the directory being listed.
    pub path: String,
    /// Parent directory, or `None` at the output root (can't ascend past it).
    pub parent: Option<String>,
    /// Path relative to the output root, forward-slashed ("" = root, "group/ive").
    pub rel: String,
    /// Immediate child folders, sorted by media count desc then name.
    pub folders: Vec<FolderEntry>,
}

/// The full snapshot returned by `open_session`. The inbox is only sent in full
/// here; mutating commands return deltas instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    pub input: String,
    pub output: String,
    pub inbox: Vec<MediaItemDto>,
    pub destinations: Vec<DestinationDto>,
}

/// The result of a single mutating operation. Carries a minimal delta so the
/// frontend can update its local inbox without re-fetching everything.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpOutcome {
    pub message: String,
    pub kind: OpKind,
    pub source_path: String,
    pub resolved_path: String,
    /// True => the frontend should drop the source row from the inbox.
    pub source_removed: bool,
    /// On undo, the item to re-insert into the inbox (if a move/trash was reversed).
    pub restored_item: Option<MediaItemDto>,
    pub can_undo: bool,
    /// Destinations with refreshed media counts (cheap; inbox not resent).
    pub destinations: Vec<DestinationDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpKind {
    Move,
    Copy,
    Trash,
    Undo,
}

/// Classify a path by extension. Mirrors the TUI's supported set.
pub fn media_kind(path: &Path) -> MediaKind {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    match ext.as_deref() {
        Some("jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" | "avif") => MediaKind::Image,
        Some("mp4" | "mkv" | "webm" | "mov" | "m4v") => MediaKind::Video,
        _ => MediaKind::Other,
    }
}

pub fn system_time_to_ms(t: SystemTime) -> Option<i64> {
    t.duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as i64)
}

/// Public alias used by sibling modules that build DTOs from `fs::Metadata`.
pub fn system_time_to_ms_pub(t: SystemTime) -> Option<i64> {
    system_time_to_ms(t)
}

/// The reserved state directory under the output root.
pub const STATE_DIR: &str = ".comfysort";
/// Trash lives under the state dir so it never appears as a destination.
pub fn trash_dir(output_root: &Path) -> PathBuf {
    output_root.join(STATE_DIR).join(".trash")
}
/// The append-only operation journal.
pub fn journal_path(output_root: &Path) -> PathBuf {
    output_root.join(STATE_DIR).join("journal.jsonl")
}
