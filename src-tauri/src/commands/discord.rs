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
        // app_id と discord_client の Arc を短時間で取り出す（I/O 中は AppStateInner の Mutex を保持しない）
        let (app_id, discord_client) = {
            let inner = arc.lock().unwrap();
            (
                inner.settings.discord_app_id.clone(),
                Arc::clone(&inner.discord_client),
            )
        };

        let mut new_client = DiscordRpcClient::new(app_id);
        let connect_result = new_client.connect();

        if connect_result.is_ok() {
            let mut client = discord_client.lock().unwrap();
            client.disconnect();
            *client = new_client;
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
    let discord_client = {
        let inner = state.0.lock().unwrap();
        Arc::clone(&inner.discord_client)
    };

    tokio::task::spawn_blocking(move || {
        discord_client.lock().unwrap().disconnect();
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
    // AppStateInner の Mutex と discord_client の Mutex を同時保持しないよう分けて取得する
    let (discord_client, error) = {
        let inner = state.0.lock().unwrap();
        (
            Arc::clone(&inner.discord_client),
            inner.discord_status.error.clone(),
        )
    };
    let connected = discord_client.lock().unwrap().is_connected();
    DiscordStatus { connected, error }
}
