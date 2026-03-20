use keyring::Entry;
use tauri::{AppHandle, Emitter};
use tauri_plugin_opener::OpenerExt;

use crate::models::status::AuthStatus;
use crate::services::lastfm::LastfmClient;
use crate::state::AppState;

const KEYRING_SERVICE: &str = "scrobcord";
const KEYRING_SESSION_KEY: &str = "lastfm_session_key";
const KEYRING_API_SECRET: &str = "lastfm_api_secret";

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

/// OS キーチェーンに api_secret を保存する
pub fn store_api_secret(secret: &str) -> Result<(), String> {
    Entry::new(KEYRING_SERVICE, KEYRING_API_SECRET)
        .map_err(|e| e.to_string())?
        .set_password(secret)
        .map_err(|e| e.to_string())
}

/// OS キーチェーンから api_secret を読む
pub fn load_api_secret() -> Option<String> {
    Entry::new(KEYRING_SERVICE, KEYRING_API_SECRET)
        .ok()
        .and_then(|e| e.get_password().ok())
}

// ---------------------------------------------------------------------------
// Tauri コマンド
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn lastfm_get_auth_token(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let (api_key, api_secret) = {
        let inner = state.0.lock().unwrap();
        (
            inner.settings.lastfm_api_key.clone(),
            // keyring 優先、なければ Settings のフィールド（まだ保存前）
            load_api_secret()
                .or_else(|| {
                    let s = inner.settings.lastfm_api_secret.clone();
                    if s.is_empty() {
                        None
                    } else {
                        Some(s)
                    }
                })
                .unwrap_or_default(),
        )
    };

    if api_key.is_empty() {
        return Err("Last.fm API Key が設定されていません".to_string());
    }
    if api_secret.is_empty() {
        return Err("Last.fm API Secret が設定されていません".to_string());
    }

    let client = LastfmClient::new(api_key.clone(), api_secret);
    let token = client.get_token().await?;

    // ブラウザで認証 URL を開く
    let auth_url = format!(
        "https://www.last.fm/api/auth/?api_key={}&token={}",
        api_key, token
    );
    app.opener()
        .open_url(&auth_url, None::<&str>)
        .map_err(|e| e.to_string())?;

    Ok(token)
}

#[tauri::command]
pub async fn lastfm_get_session(
    token: String,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let (api_key, api_secret) = {
        let inner = state.0.lock().unwrap();
        (
            inner.settings.lastfm_api_key.clone(),
            load_api_secret()
                .or_else(|| {
                    let s = inner.settings.lastfm_api_secret.clone();
                    if s.is_empty() {
                        None
                    } else {
                        Some(s)
                    }
                })
                .unwrap_or_default(),
        )
    };

    let client = LastfmClient::new(api_key, api_secret);
    let session_key = client.get_session(&token).await?;

    // keyring に保存（平文ファイルには書かない）
    store_session_key(&session_key)?;

    // ユーザー名を取得して状態を更新
    // user.getRecentTracks で username を確認する代わりに、
    // 設定の lastfm_username を使う（空なら "unknown" として後でポーリングで確定）
    let username = { state.0.lock().unwrap().settings.lastfm_username.clone() };

    {
        let mut inner = state.0.lock().unwrap();
        inner.auth_status = crate::models::status::AuthStatus {
            authenticated: true,
            username: if username.is_empty() {
                None
            } else {
                Some(username.clone())
            },
        };
    }

    app.emit(
        "lastfm-status-changed",
        crate::models::status::AuthStatus {
            authenticated: true,
            username: if username.is_empty() {
                None
            } else {
                Some(username)
            },
        },
    )
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
    } else if !has_session {
        inner.auth_status.authenticated = false;
        inner.auth_status.username = None;
    }
    inner.auth_status.clone()
}
