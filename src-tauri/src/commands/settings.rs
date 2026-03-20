use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_store::StoreExt;

use crate::models::settings::Settings;
use crate::state::AppState;

const STORE_PATH: &str = "settings.json";
const STORE_KEY: &str = "settings";

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
        autostart.enable().map_err(|e| format!("autostart enable: {e}"))?;
    } else {
        autostart.disable().map_err(|e| format!("autostart disable: {e}"))?;
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
