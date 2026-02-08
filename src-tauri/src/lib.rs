mod db;
mod models;
mod commands;

use commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      get_global_config,
      save_global_config,
      create_project,
      open_project,
      list_projects,
      check_project_name_exists,
      update_project_name,
      save_generated_data,
      get_storyboards,
      get_characters,
      get_scenes,
      get_props,
      save_chat_message,
      get_chat_history,
      call_ai_api,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
