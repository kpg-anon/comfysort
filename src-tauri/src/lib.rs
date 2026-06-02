mod commands;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::pick_directory,
            commands::open_session,
            commands::move_item,
            commands::copy_item,
            commands::move_to_hotkey,
            commands::trash_item,
            commands::create_folder,
            commands::undo,
            commands::list_folders,
            commands::delete_folder,
            commands::search_folders,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
