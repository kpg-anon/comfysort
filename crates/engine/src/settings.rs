//! Persistent user settings, stored as `config.toml` in the app config dir.
//!
//! The same `Settings` struct is the on-disk TOML schema *and* the JSON DTO sent
//! across the IPC boundary, so it serializes with `camelCase` keys (camelCase in
//! `config.toml` is intentional). Only `collision_policy` has backend behavior;
//! the rest are stored here as the single source of truth and read by the
//! frontend. Every field has a serde default so a partial or older file still
//! loads, and `load` never errors — a missing or corrupt file yields defaults.

use crate::domain::CollisionPolicy;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default = "default_collision_policy")]
    pub collision_policy: CollisionPolicy,
    #[serde(default = "default_true")]
    pub confirm_folder_delete: bool,
    #[serde(default = "default_true")]
    pub confirm_cross_drive: bool,
    #[serde(default = "default_sort_field")]
    pub default_sort_field: String,
    #[serde(default = "default_sort_order")]
    pub default_sort_order: String,
    #[serde(default = "default_filter")]
    pub default_filter: String,
    #[serde(default = "default_true")]
    pub video_autoplay: bool,
    #[serde(default = "default_true")]
    pub video_loop: bool,
    #[serde(default = "default_true")]
    pub video_muted: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Optional default inbox/destination. When both are set, the app opens
    /// straight into them and skips the start screen. Empty = unset.
    #[serde(default)]
    pub default_input: String,
    #[serde(default)]
    pub default_output: String,
}

fn default_collision_policy() -> CollisionPolicy {
    CollisionPolicy::Rename
}
fn default_theme() -> String {
    "comfy-dark".to_owned()
}
fn default_true() -> bool {
    true
}
fn default_sort_field() -> String {
    "mod".to_owned()
}
fn default_sort_order() -> String {
    "desc".to_owned()
}
fn default_filter() -> String {
    "all".to_owned()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            collision_policy: default_collision_policy(),
            confirm_folder_delete: true,
            confirm_cross_drive: true,
            default_sort_field: default_sort_field(),
            default_sort_order: default_sort_order(),
            default_filter: default_filter(),
            video_autoplay: true,
            video_loop: true,
            video_muted: true,
            theme: default_theme(),
            default_input: String::new(),
            default_output: String::new(),
        }
    }
}

/// Read and parse settings from `path`. A missing or corrupt file (or any read
/// error) falls back to `Settings::default()` — this never errors, so the app
/// always has usable settings.
pub fn load(path: &Path) -> Settings {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Settings::default();
    };
    toml::from_str(&text).unwrap_or_default()
}

/// Serialize `settings` to TOML and write it to `path` atomically: write to a
/// sibling `.tmp` file, then rename over the target so a crash mid-write can
/// never leave a half-written config behind. Creates the parent dir if needed.
pub fn save(path: &Path, settings: &Settings) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = toml::to_string_pretty(settings)?;
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, text.as_bytes())?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn missing_file_yields_defaults() {
        let dir = tempdir().unwrap();
        let settings = load(&dir.path().join("nope.toml"));
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn corrupt_file_yields_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, b"this is not = valid toml = [[[").unwrap();
        assert_eq!(load(&path), Settings::default());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sub").join("config.toml");
        let settings = Settings {
            collision_policy: CollisionPolicy::Overwrite,
            confirm_folder_delete: false,
            confirm_cross_drive: false,
            default_sort_field: "size".to_owned(),
            default_sort_order: "asc".to_owned(),
            default_filter: "videos".to_owned(),
            video_autoplay: false,
            video_loop: false,
            video_muted: false,
            theme: "nord".to_owned(),
            default_input: "C:/in".to_owned(),
            default_output: "C:/out".to_owned(),
        };
        save(&path, &settings).unwrap();
        assert!(path.exists(), "config.toml written");
        let loaded = load(&path);
        assert_eq!(loaded, settings);
    }

    #[test]
    fn partial_file_fills_missing_with_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        // Only one field present; the rest must fall back to defaults.
        std::fs::write(&path, b"collisionPolicy = \"skip\"\n").unwrap();
        let loaded = load(&path);
        assert_eq!(loaded.collision_policy, CollisionPolicy::Skip);
        assert_eq!(loaded.default_sort_field, "mod");
        assert!(loaded.video_autoplay);
    }

    #[test]
    fn collision_policy_serializes_lowercase() {
        let settings = Settings {
            collision_policy: CollisionPolicy::Skip,
            ..Default::default()
        };
        let text = toml::to_string_pretty(&settings).unwrap();
        assert!(text.contains("collisionPolicy = \"skip\""), "{text}");
    }
}
