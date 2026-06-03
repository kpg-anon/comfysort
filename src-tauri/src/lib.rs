mod commands;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init());

    // The updater is desktop-only (and the crate isn't built for mobile).
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }

    builder
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
            commands::rescan_inbox,
            commands::delete_folder,
            commands::search_folders,
            commands::bind_folder,
            commands::unbind_hotkey,
            commands::would_cross_volume,
            commands::disk_space,
            commands::get_settings,
            commands::config_file_path,
            commands::set_settings,
            commands::set_collision_policy,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
