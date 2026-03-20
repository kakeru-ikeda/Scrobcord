// Phase 2 で実装
use crate::models::status::AuthStatus;
use crate::state::AppState;

#[tauri::command]
pub async fn lastfm_get_auth_token(
    _state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    Err("Not implemented: Phase 2".to_string())
}

#[tauri::command]
pub async fn lastfm_get_session(
    _token: String,
    _state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    Err("Not implemented: Phase 2".to_string())
}

#[tauri::command]
pub async fn lastfm_logout(
    _state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    Err("Not implemented: Phase 2".to_string())
}

#[tauri::command]
pub fn lastfm_get_auth_status(state: tauri::State<'_, AppState>) -> AuthStatus {
    state.0.lock().unwrap().auth_status.clone()
}
