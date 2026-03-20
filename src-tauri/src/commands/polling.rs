use std::sync::Arc;

use tauri::AppHandle;

use crate::models::track::Track;
use crate::state::AppState;

#[tauri::command]
pub async fn start_polling(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let arc = Arc::clone(&state.0);
    {
        let mut inner = arc.lock().unwrap();
        // 既に動いていれば停止してから再起動
        if let Some(token) = inner.poll_cancel_token.take() {
            token.cancel();
        }
    }

    let arc2 = Arc::clone(&state.0);
    let token = crate::services::poller::start(app, arc2);
    state.0.lock().unwrap().poll_cancel_token = Some(token);

    Ok(())
}

#[tauri::command]
pub async fn stop_polling(state: tauri::State<'_, AppState>) -> Result<(), String> {
    if let Some(token) = state.0.lock().unwrap().poll_cancel_token.take() {
        token.cancel();
    }
    Ok(())
}

#[tauri::command]
pub fn get_now_playing(state: tauri::State<'_, AppState>) -> Option<Track> {
    state.0.lock().unwrap().now_playing.clone()
}

#[tauri::command]
pub fn get_polling_status(state: tauri::State<'_, AppState>) -> bool {
    state.0.lock().unwrap().poll_cancel_token.is_some()
}
