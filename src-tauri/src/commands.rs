//! Tauri command surface — the only bridge between the engine and the frontend.
//! Each command locks the managed session, delegates to the engine, and maps
//! `anyhow` errors to strings the frontend can display.

use comfysort_engine::domain::{DestinationDto, FolderListing, OpOutcome, SessionView};
use comfysort_engine::session::Session;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;
use tauri_plugin_dialog::DialogExt;

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
