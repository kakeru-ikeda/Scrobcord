use std::sync::Arc;

use tauri::{AppHandle, Emitter};

use crate::models::status::DiscordStatus;
use crate::services::discord_rpc::DiscordRpcClient;
use crate::state::AppState;

#[tauri::command]
pub async fn discord_connect(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let arc = Arc::clone(&state.0);

    let result = tokio::task::spawn_blocking(move || {
        // app_id だけ先に取り出し、重い接続処理中は mutex を保持しない
        let app_id = {
            let inner = arc.lock().unwrap();
            inner.settings.discord_app_id.clone()
        };

        let mut new_client = DiscordRpcClient::new(app_id);
        let connect_result = new_client.connect();

        if connect_result.is_ok() {
            let mut inner = arc.lock().unwrap();
            inner.discord_client.disconnect();
            inner.discord_client = new_client;
        }

        connect_result
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
