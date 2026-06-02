//! Recursive fuzzy folder search under the output root.
//!
//! Ported from the TUI's `navigator::fuzzy_score` — an O(n·m) DP alignment that
//! rewards consecutive runs and word-boundary matches so typing the start of a
//! folder name beats the same letters scattered across a path.

use std::path::Path;

/// Per-character match reward at candidate position `i`. Consecutive matches and
/// matches at a word boundary (`/ \ _ - space .`) score much higher, so the
/// start of a folder name outranks scattered letters.
fn char_bonus(candidate: &[char], i: usize, consecutive: bool) -> i32 {
    let mut bonus = 1;
    if consecutive {
        bonus += 10;
    }
    let at_boundary = i == 0 || matches!(candidate[i - 1], '/' | '\\' | '_' | '-' | ' ' | '.');
    if at_boundary {
        bonus += 8;
    }
    if i == 0 {
        bonus += 4;
    }
    bonus
}

/// Fuzzy score: `Some` when every char of `query` appears in `candidate` in
/// order, higher is better. Finds the *best* alignment via dynamic programming,
/// so a consecutive substring is rewarded even when those characters also appear
/// scattered earlier in the path. O(n·m) over candidate/query lengths.
pub fn fuzzy_score(candidate: &str, query: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }
    let chars: Vec<char> = candidate.chars().collect();
    let query: Vec<char> = query.chars().collect();
    if query.len() > chars.len() {
        return None;
    }

    let mut prev: Vec<Option<i32>> = vec![None; chars.len()];
    for (j, &query_char) in query.iter().enumerate() {
        let mut cur: Vec<Option<i32>> = vec![None; chars.len()];
        let mut best_gap: Option<i32> = None;
        for i in 0..chars.len() {
            if chars[i] == query_char {
                cur[i] = if j == 0 {
                    Some(char_bonus(&chars, i, false))
                } else {
                    let from_consecutive = i
                        .checked_sub(1)
                        .and_then(|k| prev[k])
                        .map(|s| s + char_bonus(&chars, i, true));
                    let from_gap = best_gap.map(|s| s + char_bonus(&chars, i, false));
                    match (from_consecutive, from_gap) {
                        (Some(a), Some(b)) => Some(a.max(b)),
                        (Some(a), None) => Some(a),
                        (None, Some(b)) => Some(b),
                        (None, None) => None,
                    }
                };
            }
            if let Some(s) = i.checked_sub(1).and_then(|k| prev[k]) {
                best_gap = Some(best_gap.map_or(s, |b| b.max(s)));
            }
        }
        prev = cur;
    }

    let raw = prev.iter().filter_map(|&score| score).max()?;
    // Scale match score above the length tiebreak so shorter candidates only win
    // when match quality is otherwise equal.
    Some(raw * 100 - chars.len() as i32)
}

/// A scored folder found by [`walk`], carrying the path relative to the output
/// root (forward-slashed) for display and re-scoring.
pub struct ScoredFolder {
    pub path: std::path::PathBuf,
    /// Forward-slashed path relative to the output root.
    pub rel: String,
    pub score: i32,
}

/// Recursively collect every folder under `root` (skipping the `.comfysort`
/// state dir) that fuzzy-matches `query`, scored against its forward-slashed
/// relative path. Unsorted; caller ranks and truncates.
pub fn walk(root: &Path, state_dir: &str, query: &str) -> Vec<ScoredFolder> {
    let lowered = query.to_ascii_lowercase();
    let mut out = Vec::new();
    collect(root, root, state_dir, &lowered, &mut out);
    out
}

fn collect(
    root: &Path,
    current: &Path,
    state_dir: &str,
    lowered_query: &str,
    out: &mut Vec<ScoredFolder>,
) {
    let Ok(entries) = std::fs::read_dir(current) else {
        return;
    };
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        if entry
            .file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case(state_dir)
        {
            continue;
        }
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .map(|r| r.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| path.to_string_lossy().into_owned());
        if let Some(score) = fuzzy_score(&rel.to_ascii_lowercase(), lowered_query) {
            out.push(ScoredFolder {
                rel,
                score,
                path: path.clone(),
            });
        }
        collect(root, &path, state_dir, lowered_query, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_score_orders_consecutive_matches_higher() {
        let consecutive = fuzzy_score("wonyoung", "wony").expect("match");
        let scattered = fuzzy_score("walk_on_yonsei_uni", "wony").expect("match");
        assert!(consecutive > scattered, "{consecutive} should beat {scattered}");
    }

    #[test]
    fn fuzzy_score_returns_none_when_chars_out_of_order() {
        assert!(fuzzy_score("wonyoung", "ynow").is_none());
    }

    #[test]
    fn fuzzy_prefers_consecutive_substring_over_scattered_path_chars() {
        // Typing `ryum` should rank `ryumommy` (a consecutive substring) above a
        // path where r/y/u/m only appear scattered.
        let target = fuzzy_score("groups/itzy/ryumommy", "ryum").expect("match");
        let scattered = fuzzy_score("r-y-u-m-scattered/path", "ryum").expect("match");
        assert!(
            target > scattered,
            "ryumommy ({target}) should beat scattered ({scattered})"
        );
    }
}
