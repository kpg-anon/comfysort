//! Tauri command surface — the only bridge between the engine and the frontend.
//! Each command locks the managed session, delegates to the engine, and maps
//! `anyhow` errors to strings the frontend can display.

use comfysort_engine::domain::{
    CollisionPolicy, DestinationDto, FolderEntry, FolderListing, MediaItemDto, OpOutcome,
    SessionView,
};
use comfysort_engine::session::Session;
use comfysort_engine::settings::{self, Settings};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;

/// Absolute path to the persisted `config.toml` in the app's config dir.
fn config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("could not resolve app config dir: {e}"))?;
    Ok(dir.join("config.toml"))
}

/// Free + total bytes on the volume holding a path (for the footer readout).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskSpace {
    pub free_bytes: u64,
    pub total_bytes: u64,
}

/// Disk space for the volume containing `path`. Returns `None` if the path
/// can't be queried (e.g. an unmapped drive), so the UI just hides the readout.
#[tauri::command]
pub async fn disk_space(path: String) -> Option<DiskSpace> {
    let p = std::path::Path::new(&path);
    let free = fs2::available_space(p).ok()?;
    let total = fs2::total_space(p).ok()?;
    Some(DiskSpace {
        free_bytes: free,
        total_bytes: total,
    })
}

/// Managed application state. `None` until a session is opened.
#[derive(Default)]
pub struct AppState {
    pub session: Mutex<Option<Session>>,
}

type CmdResult<T> = Result<T, String>;

fn with_session<T>(
    state: &State<'_, AppState>,
    f: impl FnOnce(&mut Session) -> anyhow::Result<T>,
) -> CmdResult<T> {
    let mut guard = state.session.lock().map_err(|_| "session lock poisoned")?;
    let session = guard
        .as_mut()
        .ok_or_else(|| "no session is open".to_string())?;
    f(session).map_err(|e| e.to_string())
}

/// Open a native folder-picker. Async + non-blocking: the blocking variant runs
/// on the main thread and freezes the window (white title bar) while the dialog
/// is open, so we use the callback API and await the result over a channel.
#[tauri::command]
pub async fn pick_directory(app: tauri::AppHandle, title: Option<String>) -> Option<String> {
    let (tx, mut rx) = tauri::async_runtime::channel(1);
    let mut builder = app.dialog().file();
    if let Some(title) = title {
        builder = builder.set_title(title);
    }
    builder.pick_folder(move |path| {
        let _ = tx.blocking_send(path);
    });
    rx.recv()
        .await
        .flatten()
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn open_session(
    state: State<'_, AppState>,
    input: String,
    output: String,
) -> CmdResult<SessionView> {
    let (session, view) = Session::open(PathBuf::from(input), PathBuf::from(output))
        .map_err(|e| e.to_string())?;
    *state.session.lock().map_err(|_| "session lock poisoned")? = Some(session);
    Ok(view)
}

#[tauri::command]
pub async fn move_item(
    state: State<'_, AppState>,
    source: String,
    dest_dir: String,
) -> CmdResult<OpOutcome> {
    with_session(&state, |s| {
        s.move_item(&PathBuf::from(&source), &PathBuf::from(&dest_dir))
    })
}

#[tauri::command]
pub async fn copy_item(
    state: State<'_, AppState>,
    source: String,
    dest_dir: String,
) -> CmdResult<OpOutcome> {
    with_session(&state, |s| {
        s.copy_item(&PathBuf::from(&source), &PathBuf::from(&dest_dir))
    })
}

#[tauri::command]
pub async fn move_to_hotkey(
    state: State<'_, AppState>,
    source: String,
    hotkey: String,
) -> CmdResult<OpOutcome> {
    with_session(&state, |s| {
        let src = PathBuf::from(&source);
        if hotkey == "0" {
            return s.trash_item(&src);
        }
        let dir = s
            .dest_dir_for_hotkey(&hotkey)
            .ok_or_else(|| anyhow::anyhow!("no destination bound to {hotkey}"))?;
        s.move_item(&src, &dir)
    })
}

#[tauri::command]
pub async fn trash_item(state: State<'_, AppState>, source: String) -> CmdResult<OpOutcome> {
    with_session(&state, |s| s.trash_item(&PathBuf::from(&source)))
}

#[tauri::command]
pub async fn create_folder(
    state: State<'_, AppState>,
    parent: String,
    name: String,
) -> CmdResult<DestinationDto> {
    with_session(&state, |s| {
        s.create_folder(&PathBuf::from(&parent), &name)
    })
}

#[tauri::command]
pub async fn undo(state: State<'_, AppState>) -> CmdResult<OpOutcome> {
    with_session(&state, |s| s.undo())
}

#[tauri::command]
pub async fn list_folders(state: State<'_, AppState>, path: String) -> CmdResult<FolderListing> {
    with_session(&state, |s| s.list_folders(&PathBuf::from(&path)))
}

/// Re-scan the input directory (e.g. after external changes) and return the
/// fresh inbox. Destinations are left untouched.
#[tauri::command]
pub async fn rescan_inbox(state: State<'_, AppState>) -> CmdResult<Vec<MediaItemDto>> {
    with_session(&state, |s| s.rescan_inbox())
}

#[tauri::command]
pub async fn delete_folder(state: State<'_, AppState>, path: String) -> CmdResult<OpOutcome> {
    with_session(&state, |s| s.delete_folder(&PathBuf::from(&path)))
}

#[tauri::command]
pub async fn search_folders(
    state: State<'_, AppState>,
    query: String,
) -> CmdResult<Vec<FolderEntry>> {
    with_session(&state, |s| Ok(s.search_folders(&query)))
}

/// Bind a folder (under the output subtree) to a single-char hotkey. Persists
/// the binding and returns the refreshed destination list.
#[tauri::command]
pub async fn bind_folder(
    state: State<'_, AppState>,
    path: String,
    hotkey: String,
) -> CmdResult<Vec<DestinationDto>> {
    let key = hotkey
        .chars()
        .next()
        .ok_or_else(|| "hotkey is empty".to_string())?;
    with_session(&state, |s| s.bind_folder(&PathBuf::from(&path), key))
}

/// Clear a hotkey binding and return the refreshed destination list.
#[tauri::command]
pub async fn unbind_hotkey(
    state: State<'_, AppState>,
    hotkey: String,
) -> CmdResult<Vec<DestinationDto>> {
    let key = hotkey
        .chars()
        .next()
        .ok_or_else(|| "hotkey is empty".to_string())?;
    with_session(&state, |s| s.unbind_hotkey(key))
}

/// Whether moving `source` into `dest_dir` would cross a volume boundary, so
/// the frontend can show a confirm modal before the move.
#[tauri::command]
pub fn would_cross_volume(source: String, dest_dir: String) -> bool {
    comfysort_engine::operations::is_cross_volume(
        &PathBuf::from(&source),
        &PathBuf::from(&dest_dir),
    )
}

/// Load the persisted settings from `config.toml`. Never fails on a missing or
/// corrupt file — the engine returns defaults — so this only errors if the
/// config dir itself can't be resolved.
#[tauri::command]
pub fn get_settings(app: tauri::AppHandle) -> CmdResult<Settings> {
    Ok(settings::load(&config_path(&app)?))
}

/// Persist the full settings struct to `config.toml` (atomically). This is the
/// single source of truth for both backend behavior and the frontend's overlay.
#[tauri::command]
pub fn set_settings(app: tauri::AppHandle, settings: Settings) -> CmdResult<()> {
    let path = config_path(&app)?;
    settings::save(&path, &settings).map_err(|e| e.to_string())
}

/// Apply the collision policy to the live session (if one is open). Parses the
/// frontend's lowercase string into the engine enum; a no-op `Ok(())` when no
/// session is open. Persisting the choice is the frontend's job via `set_settings`.
#[tauri::command]
pub fn set_collision_policy(state: State<'_, AppState>, policy: String) -> CmdResult<()> {
    let parsed = match policy.as_str() {
        "rename" => CollisionPolicy::Rename,
        "skip" => CollisionPolicy::Skip,
        "overwrite" => CollisionPolicy::Overwrite,
        other => return Err(format!("unknown collision policy: {other}")),
    };
    let mut guard = state.session.lock().map_err(|_| "session lock poisoned")?;
    if let Some(session) = guard.as_mut() {
        session.set_collision_policy(parsed);
    }
    Ok(())
}
