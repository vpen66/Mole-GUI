mod commands;
mod mole;

use commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            get_mole_version,
            get_free_space_kb,
            get_system_status,
            clean_dry_run,
            clean_execute,
            uninstall_scan_apps,
            uninstall_execute,
            purge_dry_run,
            purge_execute,
            optimize_dry_run,
            optimize_execute,
            analyze_scan,
            analyze_delete,
            get_history,
            check_sudo_session,
            request_sudo_session,
            stop_sudo_session,
            get_mole_path_config,
            set_mole_path_config,
        ])
        .setup(|app| {
            let _window = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("Mole")
            .inner_size(1100.0, 750.0)
            .min_inner_size(800.0, 600.0)
            .resizable(true)
            .center()
            .decorations(true)
            .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
