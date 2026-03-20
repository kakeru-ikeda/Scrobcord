use std::sync::Arc;

use tauri::{AppHandle, Emitter};

use crate::models::status::DiscordStatus;
use crate::state::AppState;

#[tauri::command]
pub async fn discord_connect(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let arc = Arc::clone(&state.0);

    let result = tokio::task::spawn_blocking(move || {
        let mut inner = arc.lock().unwrap();
        // app_id を settings から同期
        let app_id = inner.settings.discord_app_id.clone();
        inner.discord_client.app_id = app_id;
        inner.discord_client.connect()
    })
    .await
    .map_err(|e| e.to_string())?;

    let status = match &result {
        Ok(()) => DiscordStatus {
            connected: true,
            error: None,
        },
        Err(e) => DiscordStatus {
            connected: false,
            error: Some(e.clone()),
        },
    };
    state.0.lock().unwrap().discord_status = status.clone();
    app.emit("discord-status-changed", &status)
        .map_err(|e| e.to_string())?;

    result
}

#[tauri::command]
pub async fn discord_disconnect(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let arc = Arc::clone(&state.0);

    tokio::task::spawn_blocking(move || {
        arc.lock().unwrap().discord_client.disconnect();
    })
    .await
    .map_err(|e| e.to_string())?;

    let status = DiscordStatus {
        connected: false,
        error: None,
    };
    state.0.lock().unwrap().discord_status = status.clone();
    app.emit("discord-status-changed", &status)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn discord_get_status(state: tauri::State<'_, AppState>) -> DiscordStatus {
    let inner = state.0.lock().unwrap();
    DiscordStatus {
        connected: inner.discord_client.is_connected(),
        error: inner.discord_status.error.clone(),
    }
}
