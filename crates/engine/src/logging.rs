//! Append-only debug logger.
//!
//! Best-effort diagnostics written to `<output_root>/.comfysort/comfysort.log`.
//! Logging must never crash a sort session, so all errors are swallowed (after
//! an `eprintln!`) rather than propagated.

use crate::domain::STATE_DIR;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

/// Append `"<RFC3339 timestamp> <msg>\n"` to the per-output debug log. Creates
/// the `.comfysort` state directory if needed. Never panics; on any I/O error
/// it reports to stderr and returns.
pub fn log(output_root: &Path, msg: &str) {
    let state_dir = output_root.join(STATE_DIR);
    if let Err(err) = std::fs::create_dir_all(&state_dir) {
        eprintln!("comfysort: could not create log dir {}: {err}", state_dir.display());
        return;
    }
    let log_path = state_dir.join("comfysort.log");
    let line = format!("{} {msg}\n", chrono::Utc::now().to_rfc3339());
    match OpenOptions::new().create(true).append(true).open(&log_path) {
        Ok(mut file) => {
            if let Err(err) = file.write_all(line.as_bytes()) {
                eprintln!("comfysort: log write failed: {err}");
            }
        }
        Err(err) => {
            eprintln!("comfysort: could not open log {}: {err}", log_path.display());
        }
    }
}
