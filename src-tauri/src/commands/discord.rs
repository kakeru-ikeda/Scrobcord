// Phase 3 で実装
use crate::models::status::DiscordStatus;
use crate::state::AppState;

#[tauri::command]
pub async fn discord_connect(
    _state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    Err("Not implemented: Phase 3".to_string())
}

#[tauri::command]
pub async fn discord_disconnect(
    _state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    Err("Not implemented: Phase 3".to_string())
}

#[tauri::command]
pub fn discord_get_status(state: tauri::State<'_, AppState>) -> DiscordStatus {
    state.0.lock().unwrap().discord_status.clone()
}
