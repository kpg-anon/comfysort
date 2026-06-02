//! Per-output-root persistence for user-initiated hotkey bindings.
//!
//! Anything the user binds at runtime is stored under
//! `<output_root>/.comfysort/bindings.json` so subsequent sessions on the same
//! output tree restore those hotkeys automatically.
//!
//! Paths are stored relative to the output root so a tree can be relocated
//! without losing its bindings.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const CURRENT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistedBindings {
    #[serde(default = "default_version")]
    pub version: u32,
    /// Hotkey character → path relative to the output root.
    #[serde(default)]
    pub hotkeys: BTreeMap<String, PathBuf>,
}

fn default_version() -> u32 {
    CURRENT_VERSION
}

impl Default for PersistedBindings {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION,
            hotkeys: BTreeMap::new(),
        }
    }
}

impl PersistedBindings {
    /// Load saved bindings for this output root. Missing or unreadable files
    /// return an empty set rather than failing — the app should be usable on
    /// the first launch of a fresh output tree.
    pub fn load(output_root: &Path) -> Self {
        let path = bindings_path(output_root);
        let Ok(bytes) = fs::read(&path) else {
            return Self::default();
        };
        serde_json::from_slice(&bytes).unwrap_or_default()
    }

    /// Persist the current set of user-bound hotkeys atomically. Writes
    /// through a sibling `.tmp` file so a kill mid-write can't corrupt the
    /// on-disk state.
    pub fn save(&self, output_root: &Path) -> Result<()> {
        let path = bindings_path(output_root);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(self)?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, bytes)?;
        fs::rename(&tmp, &path)?;
        Ok(())
    }

    /// Resolve persisted entries into `(hotkey, absolute_path)` pairs so the
    /// caller can apply them against the in-memory destinations list.
    pub fn resolved(&self, output_root: &Path) -> Vec<(char, PathBuf)> {
        let mut out = Vec::with_capacity(self.hotkeys.len());
        for (key, relative) in &self.hotkeys {
            let Some(hotkey) = key.chars().next() else {
                continue;
            };
            if key.chars().count() != 1 {
                continue;
            }
            let absolute = if relative.is_absolute() {
                relative.clone()
            } else {
                output_root.join(relative)
            };
            out.push((hotkey, absolute));
        }
        out
    }

    pub fn set(&mut self, hotkey: char, absolute_path: &Path, output_root: &Path) {
        let relative = absolute_path
            .strip_prefix(output_root)
            .map(PathBuf::from)
            .unwrap_or_else(|_| absolute_path.to_path_buf());
        self.hotkeys.insert(hotkey.to_string(), relative);
    }

    pub fn remove_hotkey(&mut self, hotkey: char) {
        self.hotkeys.remove(&hotkey.to_string());
    }

    /// Drop every entry pointing at `absolute_path` or anything nested under
    /// it. Used when a folder is deleted so its hotkey doesn't reappear on
    /// the next launch pointing at a path that no longer exists.
    pub fn remove_under(&mut self, absolute_path: &Path, output_root: &Path) {
        self.hotkeys.retain(|_, relative| {
            let candidate = if relative.is_absolute() {
                relative.clone()
            } else {
                output_root.join(relative.as_path())
            };
            !(candidate == absolute_path || candidate.starts_with(absolute_path))
        });
    }
}

pub fn bindings_path(output_root: &Path) -> PathBuf {
    output_root.join(".comfysort").join("bindings.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_returns_default_when_no_file_exists() {
        let dir = tempdir().expect("tempdir");
        let loaded = PersistedBindings::load(dir.path());
        assert!(loaded.hotkeys.is_empty());
        assert_eq!(loaded.version, CURRENT_VERSION);
    }

    #[test]
    fn save_then_load_roundtrips_relative_paths() {
        let dir = tempdir().expect("tempdir");
        let output_root = dir.path();
        let mut bindings = PersistedBindings::default();
        bindings.set('1', &output_root.join("keep").join("ive"), output_root);
        bindings.set('2', &output_root.join("group").join("karina"), output_root);
        bindings.save(output_root).expect("save");

        let loaded = PersistedBindings::load(output_root);
        let resolved: Vec<(char, PathBuf)> = loaded.resolved(output_root);
        assert!(resolved.contains(&('1', output_root.join("keep").join("ive"))));
        assert!(resolved.contains(&('2', output_root.join("group").join("karina"))));
    }

    #[test]
    fn remove_under_drops_descendants() {
        let dir = tempdir().expect("tempdir");
        let output_root = dir.path();
        let mut bindings = PersistedBindings::default();
        bindings.set('1', &output_root.join("keep").join("ive"), output_root);
        bindings.set(
            '2',
            &output_root.join("keep").join("ive").join("2024"),
            output_root,
        );
        bindings.set('3', &output_root.join("keep").join("aespa"), output_root);

        bindings.remove_under(&output_root.join("keep").join("ive"), output_root);

        assert!(bindings.hotkeys.get("1").is_none());
        assert!(bindings.hotkeys.get("2").is_none());
        assert!(
            bindings.hotkeys.get("3").is_some(),
            "sibling folder is untouched"
        );
    }

    #[test]
    fn corrupted_file_falls_back_to_empty() {
        let dir = tempdir().expect("tempdir");
        let path = bindings_path(dir.path());
        fs::create_dir_all(path.parent().unwrap()).expect("parent");
        fs::write(&path, b"not valid json").expect("write garbage");

        let loaded = PersistedBindings::load(dir.path());
        assert!(loaded.hotkeys.is_empty());
    }
}
