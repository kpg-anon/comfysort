//! Folders the user has marked as ignored.
//!
//! An ignored folder is hidden from the Navigator listing, skipped by the fuzzy
//! folder search, left out of the sort-target scan, and not counted in a
//! parent's recursive media count. Nothing on disk is touched — this is a view
//! filter over the destination tree, not an operation.
//!
//! The rules come from `ignoredFolders` in `config.toml` (and from the
//! Navigator's right-click "Ignore this folder", which appends an absolute
//! path). Each entry is one of two kinds:
//!
//! - **path rule** — the entry contains `/` or `\`. It names one absolute
//!   folder; that folder *and everything under it* is ignored.
//! - **name rule** — the entry has no separator. Any folder with that name is
//!   ignored, at any depth (e.g. `.thumbnails`).
//!
//! Matching is case-insensitive and separator-insensitive, which is what
//! Windows paths need and is harmless elsewhere.

use std::path::Path;

/// Normalize for comparison: forward slashes, no trailing separator, lowercase.
fn norm(path: &str) -> String {
    path.replace('\\', "/").trim_end_matches('/').to_lowercase()
}

/// The compiled `ignoredFolders` rules. Built once per settings change and
/// consulted by every directory walk over the output tree.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IgnoreSet {
    /// Normalized absolute paths; each ignores that folder and its subtree.
    paths: Vec<String>,
    /// Lowercased folder names; each ignores any folder so named, at any depth.
    names: Vec<String>,
}

impl IgnoreSet {
    /// Compile raw `config.toml` entries. Blank entries are dropped; the rest
    /// are split into path and name rules, normalized, and deduplicated.
    pub fn new(entries: &[String]) -> Self {
        let mut paths = Vec::new();
        let mut names = Vec::new();
        for raw in entries {
            let entry = raw.trim();
            if entry.is_empty() {
                continue;
            }
            if entry.contains('/') || entry.contains('\\') {
                let normalized = norm(entry);
                if !normalized.is_empty() {
                    paths.push(normalized);
                }
            } else {
                names.push(entry.to_lowercase());
            }
        }
        paths.sort();
        paths.dedup();
        names.sort();
        names.dedup();
        Self { paths, names }
    }

    /// True when no rules are configured — the common case, and the cheap exit
    /// every walk takes before doing any string work.
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty() && self.names.is_empty()
    }

    /// Whether `dir` is ignored: its final segment matches a name rule, or it
    /// is (or sits under) a path rule.
    pub fn is_ignored(&self, dir: &Path) -> bool {
        if self.is_empty() {
            return false;
        }
        if !self.names.is_empty() {
            if let Some(name) = dir.file_name() {
                let lowered = name.to_string_lossy().to_lowercase();
                if self.names.iter().any(|n| *n == lowered) {
                    return true;
                }
            }
        }
        if self.paths.is_empty() {
            return false;
        }
        let candidate = norm(&dir.to_string_lossy());
        self.paths
            .iter()
            .any(|rule| candidate == *rule || candidate.starts_with(&format!("{rule}/")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn set(entries: &[&str]) -> IgnoreSet {
        IgnoreSet::new(&entries.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>())
    }

    #[test]
    fn empty_set_ignores_nothing() {
        let ignores = set(&[]);
        assert!(ignores.is_empty());
        assert!(!ignores.is_ignored(&PathBuf::from("D:/out/anything")));
    }

    #[test]
    fn blank_entries_are_dropped() {
        assert!(set(&["", "   "]).is_empty());
    }

    #[test]
    fn path_rule_matches_the_folder_and_its_subtree() {
        let ignores = set(&["D:/out/_raw"]);
        assert!(ignores.is_ignored(&PathBuf::from("D:/out/_raw")));
        assert!(ignores.is_ignored(&PathBuf::from("D:/out/_raw/nested/deep")));
        assert!(!ignores.is_ignored(&PathBuf::from("D:/out/_rawish")));
        assert!(!ignores.is_ignored(&PathBuf::from("D:/out/keep")));
    }

    #[test]
    fn path_rule_is_case_and_separator_insensitive() {
        let ignores = set(&[r"D:\Out\_Raw\"]);
        assert!(ignores.is_ignored(&PathBuf::from("d:/out/_raw")));
        assert!(ignores.is_ignored(&PathBuf::from(r"D:\out\_RAW\sub")));
    }

    #[test]
    fn name_rule_matches_at_any_depth() {
        let ignores = set(&[".thumbnails"]);
        assert!(ignores.is_ignored(&PathBuf::from("D:/out/.thumbnails")));
        assert!(ignores.is_ignored(&PathBuf::from("D:/out/a/b/.Thumbnails")));
        // Only the folder itself, not what sits under it — the walkers never
        // descend into an ignored folder, so a subtree rule isn't needed here.
        assert!(!ignores.is_ignored(&PathBuf::from("D:/out/.thumbnails-old")));
    }

    #[test]
    fn duplicate_entries_collapse() {
        let ignores = set(&["D:/out/_raw", r"d:\out\_raw", "temp", "TEMP"]);
        assert_eq!(ignores, set(&["D:/out/_raw", "temp"]));
    }
}
