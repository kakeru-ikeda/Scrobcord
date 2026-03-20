// Phase 5 で実装
use crate::models::settings::Settings;
use crate::state::AppState;

#[tauri::command]
pub fn get_settings(state: tauri::State<'_, AppState>) -> Settings {
    state.0.lock().unwrap().settings.clone()
}

#[tauri::command]
pub async fn save_settings(
    _settings: Settings,
    _state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    Err("Not implemented: Phase 5".to_string())
}
