use std::path::PathBuf;
use tauri_plugin_store::StoreExt;

const STORE_PATH: &str = "settings.json";
const MOLE_PATH_KEY: &str = "mole_path";

/// Get the user-configured Mole CLI path from the store, if any.
pub fn get_configured_mole_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    let store = app.store(STORE_PATH).ok()?;
    let value = store.get(MOLE_PATH_KEY)?;
    let path_str = value.as_str()?;
    if path_str.is_empty() {
        return None;
    }
    let path = PathBuf::from(path_str);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

/// Set the user-configured Mole CLI path in the store.
/// Pass an empty string to clear the custom path and use auto-detection.
pub fn set_configured_mole_path(app: &tauri::AppHandle, path: &str) -> Result<(), String> {
    let store = app
        .store(STORE_PATH)
        .map_err(|e| format!("Failed to open settings store: {}", e))?;
    store.set(MOLE_PATH_KEY.to_string(), serde_json::Value::String(path.to_string()));
    store
        .save()
        .map_err(|e| format!("Failed to persist settings: {}", e))?;
    Ok(())
}
