use keyring::Entry;
use log::warn;
use tauri::AppHandle;
use tauri::Emitter;
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_store::StoreExt;

use crate::models::{
    settings::Settings,
    status::{AuthStatus, DiscordStatus},
};
use crate::state::AppState;

const STORE_PATH: &str = "settings.json";
const STORE_KEY: &str = "settings";
const KEYRING_SERVICE: &str = "scrobcord";
const KEYRING_SESSION_KEY: &str = "lastfm_session_key";

/// Store からアプリ起動時に Settings を読み込む（lib.rs の setup から呼ぶ）
pub fn load_settings_from_store(app: &AppHandle) -> Settings {
    let Ok(store) = app.store(STORE_PATH) else {
        return Settings::default();
    };
    store
        .get(STORE_KEY)
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

#[tauri::command]
pub fn get_settings(state: tauri::State<'_, AppState>) -> Settings {
    state.0.lock().unwrap().settings.clone()
}

#[tauri::command]
pub async fn save_settings(
    app: AppHandle,
    settings: Settings,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // AppState を更新
    let start_on_login = settings.start_on_login;
    {
        let mut inner = state.0.lock().unwrap();
        inner.settings = settings.clone();
    }

    // autostart と同期
    let autostart = app.autolaunch();
    if start_on_login {
        if let Err(e) = autostart.enable() {
            warn!("autostart enable failed (settings save continues): {e}");
        }
    } else {
        if let Err(e) = autostart.disable() {
            warn!("autostart disable failed (settings save continues): {e}");
        }
    }

    // tauri-plugin-store へ永続化
    let store = app
        .store(STORE_PATH)
        .map_err(|e| format!("store open: {e}"))?;
    store.set(
        STORE_KEY,
        serde_json::to_value(&settings).map_err(|e| e.to_string())?,
    );
    store.save().map_err(|e| format!("store save: {e}"))?;

    Ok(())
}

#[tauri::command]
pub async fn reset_saved_data(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // 1) keyring の Last.fm session を削除
    if let Ok(entry) = Entry::new(KEYRING_SERVICE, KEYRING_SESSION_KEY) {
        let _ = entry.delete_password();
    }

    // 2) autostart は無効化
    let autostart = app.autolaunch();
    if let Err(e) = autostart.disable() {
        warn!("autostart disable failed on reset (continues): {e}");
    }

    // 3) 設定をデフォルトで永続化
    let defaults = Settings::default();
    let store = app
        .store(STORE_PATH)
        .map_err(|e| format!("store open: {e}"))?;
    store.set(
        STORE_KEY,
        serde_json::to_value(&defaults).map_err(|e| e.to_string())?,
    );
    store.save().map_err(|e| format!("store save: {e}"))?;

    // 4) 実行中タスクとメモリ状態をリセット
    {
        let mut inner = state.0.lock().unwrap();

        if let Some(token) = inner.poll_cancel_token.take() {
            token.cancel();
        }

        if inner.discord_client.is_connected() {
            if let Err(e) = inner.discord_client.clear_activity() {
                warn!("clear_activity on reset: {e}");
            }
            inner.discord_client.disconnect();
        }

        inner.settings = defaults;
        inner.auth_status = AuthStatus {
            authenticated: false,
            username: None,
        };
        inner.discord_status = DiscordStatus {
            connected: false,
            error: None,
        };
        inner.now_playing = None;
        inner.pending_auth_token = None;
    }

    // 5) フロントへ状態更新通知
    app.emit(
        "lastfm-status-changed",
        AuthStatus {
            authenticated: false,
            username: None,
        },
    )
    .map_err(|e| e.to_string())?;

    app.emit(
        "discord-status-changed",
        DiscordStatus {
            connected: false,
            error: None,
        },
    )
    .map_err(|e| e.to_string())?;

    app.emit("track-changed", serde_json::json!({ "track": null }))
        .map_err(|e| e.to_string())?;

    app.emit(
        "polling-status-changed",
        serde_json::json!({ "running": false }),
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_default_values() {
        let s = Settings::default();
        assert_eq!(s.poll_interval_secs, 15);
        assert!(s.discord_enabled);
        assert!(s.rpc_show_album_art);
        assert!(s.rpc_show_timestamp);
        assert!(s.rpc_show_lastfm_button);
        assert_eq!(s.language, "ja");
    }

    #[test]
    fn settings_json_round_trip() {
        let original = Settings {
            lastfm_username: "user123".to_string(),
            discord_app_id: "123456789".to_string(),
            discord_enabled: false,
            rpc_details_format: "{track}".to_string(),
            rpc_state_format: "{artist}".to_string(),
            rpc_show_album_art: false,
            rpc_show_timestamp: false,
            rpc_show_lastfm_button: true,
            poll_interval_secs: 30,
            start_on_login: true,
            minimize_to_tray: true,
            language: "en".to_string(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.poll_interval_secs, original.poll_interval_secs);
        assert_eq!(restored.discord_enabled, original.discord_enabled);
        assert_eq!(restored.language, original.language);
    }
}
