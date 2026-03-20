use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use crate::commands::auth::store_api_secret;
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
        .map(|mut s: Settings| {
            // ストアに api_secret が万一残っていても使わない
            s.lastfm_api_secret = String::new();
            s
        })
        .unwrap_or_default()
}

#[tauri::command]
pub fn get_settings(state: tauri::State<'_, AppState>) -> Settings {
    // api_secret は keyring にあるため、返す時は空にする（UI 側でマスク表示）
    let mut s = state.0.lock().unwrap().settings.clone();
    s.lastfm_api_secret = String::new();
    s
}

#[tauri::command]
pub async fn save_settings(
    app: AppHandle,
    mut settings: Settings,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // api_secret が空でなければ keyring に保存してフィールドをクリア
    if !settings.lastfm_api_secret.is_empty() {
        store_api_secret(&settings.lastfm_api_secret)?;
        settings.lastfm_api_secret = String::new();
    }

    // AppState を更新
    {
        let mut inner = state.0.lock().unwrap();
        inner.settings = settings.clone();
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
            lastfm_api_key: "mykey".to_string(),
            lastfm_api_secret: String::new(),
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
        assert_eq!(restored.lastfm_api_key, original.lastfm_api_key);
        assert_eq!(restored.poll_interval_secs, original.poll_interval_secs);
        assert_eq!(restored.discord_enabled, original.discord_enabled);
        assert_eq!(restored.language, original.language);
    }
}
