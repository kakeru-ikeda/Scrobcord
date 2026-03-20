// Phase 5 で Store 永続化を完全実装。Phase 2 で api_secret の keyring 保存を追加済み。
use crate::commands::auth::store_api_secret;
use crate::models::settings::Settings;
use crate::state::AppState;

#[tauri::command]
pub fn get_settings(state: tauri::State<'_, AppState>) -> Settings {
    // api_secret は keyring にあるため、返す時は空にする（UI 側でマスク表示）
    let mut s = state.0.lock().unwrap().settings.clone();
    s.lastfm_api_secret = String::new();
    s
}

#[tauri::command]
pub async fn save_settings(
    mut settings: Settings,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // api_secret が空でなければ keyring に保存してフィールドをクリア
    if !settings.lastfm_api_secret.is_empty() {
        store_api_secret(&settings.lastfm_api_secret)?;
        settings.lastfm_api_secret = String::new();
    }

    {
        let mut inner = state.0.lock().unwrap();
        inner.settings = settings;
    }

    // Phase 5 で tauri-plugin-store への永続化を追加
    Ok(())
}
