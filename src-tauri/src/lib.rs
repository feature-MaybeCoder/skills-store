mod skills;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_sql::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            skills::get_state,
            skills::sync_all,
            skills::add_project,
            skills::remove_project,
            skills::import_global_skill,
            skills::set_skills_enabled,
            skills::delete_skills,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
