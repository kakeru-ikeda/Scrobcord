// Phase 4 で実装
use tauri::AppHandle;

use crate::models::track::Track;
use crate::state::AppState;

#[tauri::command]
pub async fn start_polling(
    _app: AppHandle,
    _state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    Err("Not implemented: Phase 4".to_string())
}

#[tauri::command]
pub async fn stop_polling(
    _state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    Err("Not implemented: Phase 4".to_string())
}

#[tauri::command]
pub fn get_now_playing(state: tauri::State<'_, AppState>) -> Option<Track> {
    state.0.lock().unwrap().now_playing.clone()
}
