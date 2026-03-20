use keyring::Entry;
use log::{info, warn};
use tauri::{AppHandle, Emitter};
use tauri_plugin_opener::OpenerExt;

use crate::models::status::AuthStatus;
use crate::services::lastfm::LastfmClient;
use crate::state::AppState;

const KEYRING_SERVICE: &str = "scrobcord";
const KEYRING_SESSION_KEY: &str = "lastfm_session_key";

/// OS キーチェーンから session_key を読む
pub fn load_session_key() -> Option<String> {
    Entry::new(KEYRING_SERVICE, KEYRING_SESSION_KEY)
        .ok()
        .and_then(|e| e.get_password().ok())
}

/// OS キーチェーンに session_key を保存する
fn store_session_key(key: &str) -> Result<(), String> {
    Entry::new(KEYRING_SERVICE, KEYRING_SESSION_KEY)
        .map_err(|e| e.to_string())?
        .set_password(key)
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Tauri コマンド
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn lastfm_get_auth_token(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let client = LastfmClient::new();

    if client.api_key.is_empty() {
        return Err(
            "LASTFM_API_KEY がビルド時に埋め込まれていません。環境変数を設定して再ビルドしてください"
                .to_string(),
        );
    }

    let token = client.get_token().await?;

    // トークンを AppState に保持（getSession 呼び出しまで保管）
    state.0.lock().unwrap().pending_auth_token = Some(token.clone());

    // ブラウザで認証 URL を開く
    let auth_url = format!(
        "https://www.last.fm/api/auth/?api_key={}&token={}",
        client.api_key, token
    );
    app.opener()
        .open_url(&auth_url, None::<&str>)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn lastfm_get_session(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // AppState に保存しておいた一時トークンを取り出す
    let token = state
        .0
        .lock()
        .unwrap()
        .pending_auth_token
        .clone()
        .ok_or_else(|| "先に [Last.fm でログイン] をクリックしてください".to_string())?;

    let client = LastfmClient::new();
    let session = client.get_session(&token).await?;

    // 一時トークンをクリア
    state.0.lock().unwrap().pending_auth_token = None;

    // keyring に保存（平文ファイルには書かない）
    store_session_key(&session.key)?;

    let status = crate::models::status::AuthStatus {
        authenticated: true,
        username: Some(session.username.clone()),
    };

    {
        let mut inner = state.0.lock().unwrap();
        inner.settings.lastfm_username = session.username.clone();
        inner.auth_status = status.clone();
    }

    // username を永続化し、次回起動時もポーリング可能にする
    let updated_settings = { state.0.lock().unwrap().settings.clone() };
    crate::commands::settings::save_settings(app.clone(), updated_settings, state).await?;

    app.emit("lastfm-status-changed", status)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn lastfm_logout(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // keyring から session_key を削除
    if let Ok(entry) = Entry::new(KEYRING_SERVICE, KEYRING_SESSION_KEY) {
        let _ = entry.delete_password();
    }

    {
        let mut inner = state.0.lock().unwrap();
        inner.auth_status = crate::models::status::AuthStatus {
            authenticated: false,
            username: None,
        };
    }

    app.emit(
        "lastfm-status-changed",
        crate::models::status::AuthStatus {
            authenticated: false,
            username: None,
        },
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn lastfm_get_auth_status(state: tauri::State<'_, AppState>) -> AuthStatus {
    // 起動時に keyring から session_key を確認して状態を同期する
    let has_session = load_session_key().is_some();
    let mut inner = state.0.lock().unwrap();

    if has_session && !inner.auth_status.authenticated {
        inner.auth_status.authenticated = true;
        if inner.auth_status.username.is_none() && !inner.settings.lastfm_username.is_empty() {
            inner.auth_status.username = Some(inner.settings.lastfm_username.clone());
        }
    } else if !has_session {
        inner.auth_status.authenticated = false;
        inner.auth_status.username = None;
    }

    info!(
        "lastfm auth status: authenticated={} username={:?} settings_username='{}'",
        inner.auth_status.authenticated, inner.auth_status.username, inner.settings.lastfm_username
    );

    if inner.auth_status.authenticated && inner.settings.lastfm_username.is_empty() {
        warn!("lastfm username is empty in settings; polling cannot fetch now-playing until username is set (re-login may be required)");
    }

    inner.auth_status.clone()
}
