//! Tauri command surface — the only bridge between the engine and the frontend.
//! Each command locks the managed session, delegates to the engine, and maps
//! `anyhow` errors to strings the frontend can display.

use comfysort_engine::domain::{
    DestinationDto, FolderEntry, FolderListing, OpOutcome, SessionView,
};
use comfysort_engine::session::Session;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;
use tauri_plugin_dialog::DialogExt;

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
pub fn disk_space(path: String) -> Option<DiskSpace> {
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

/// Open a native folder-picker. Blocking variant so we can return the choice.
#[tauri::command]
pub fn pick_directory(app: tauri::AppHandle, title: Option<String>) -> Option<String> {
    let mut builder = app.dialog().file();
    if let Some(title) = title {
        builder = builder.set_title(title);
    }
    builder
        .blocking_pick_folder()
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
pub fn move_item(
    state: State<'_, AppState>,
    source: String,
    dest_dir: String,
) -> CmdResult<OpOutcome> {
    with_session(&state, |s| {
        s.move_item(&PathBuf::from(&source), &PathBuf::from(&dest_dir))
    })
}

#[tauri::command]
pub fn copy_item(
    state: State<'_, AppState>,
    source: String,
    dest_dir: String,
) -> CmdResult<OpOutcome> {
    with_session(&state, |s| {
        s.copy_item(&PathBuf::from(&source), &PathBuf::from(&dest_dir))
    })
}

#[tauri::command]
pub fn move_to_hotkey(
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
pub fn trash_item(state: State<'_, AppState>, source: String) -> CmdResult<OpOutcome> {
    with_session(&state, |s| s.trash_item(&PathBuf::from(&source)))
}

#[tauri::command]
pub fn create_folder(
    state: State<'_, AppState>,
    parent: String,
    name: String,
) -> CmdResult<DestinationDto> {
    with_session(&state, |s| {
        s.create_folder(&PathBuf::from(&parent), &name)
    })
}

#[tauri::command]
pub fn undo(state: State<'_, AppState>) -> CmdResult<OpOutcome> {
    with_session(&state, |s| s.undo())
}

#[tauri::command]
pub fn list_folders(state: State<'_, AppState>, path: String) -> CmdResult<FolderListing> {
    with_session(&state, |s| s.list_folders(&PathBuf::from(&path)))
}

#[tauri::command]
pub fn delete_folder(state: State<'_, AppState>, path: String) -> CmdResult<OpOutcome> {
    with_session(&state, |s| s.delete_folder(&PathBuf::from(&path)))
}

#[tauri::command]
pub fn search_folders(
    state: State<'_, AppState>,
    query: String,
) -> CmdResult<Vec<FolderEntry>> {
    with_session(&state, |s| Ok(s.search_folders(&query)))
}

/// Bind a folder (under the output subtree) to a single-char hotkey. Persists
/// the binding and returns the refreshed destination list.
#[tauri::command]
pub fn bind_folder(
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
pub fn unbind_hotkey(
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
