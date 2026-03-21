use std::sync::{Arc, Mutex};

use keyring::Entry;
use log::{info, warn};
use tauri::{AppHandle, Emitter};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_store::StoreExt;
use tokio_util::sync::CancellationToken;

use crate::models::status::AuthStatus;
use crate::services::lastfm::{LastfmSession};
use crate::state::{AppState, AppStateInner};

const KEYRING_SERVICE: &str = "scrobcord";
const KEYRING_SESSION_KEY: &str = "lastfm_session_key";
const STORE_PATH: &str = "settings.json";
const STORE_KEY: &str = "settings";
/// 自動ポーリングのタイムアウト（秒）
const AUTH_POLL_TIMEOUT_SECS: u64 = 300;
/// ポーリング間隔（秒）
const AUTH_POLL_INTERVAL_SECS: u64 = 3;

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

/// 認証完了処理（セッション取得後の共通ロジック）
async fn complete_auth(
    app: &AppHandle,
    state_arc: &Arc<Mutex<AppStateInner>>,
    session: LastfmSession,
) -> Result<(), String> {
    store_session_key(&session.key)?;

    let status = AuthStatus {
        authenticated: true,
        username: Some(session.username.clone()),
    };

    let settings = {
        let mut inner = state_arc.lock().unwrap();
        inner.settings.lastfm_username = session.username.clone();
        inner.auth_status = status.clone();
        inner.pending_auth_token = None;
        inner.auth_poll_cancel_token = None;
        inner.settings.clone()
    };

    // username を Store へ永続化
    if let Ok(store) = app.store(STORE_PATH) {
        if let Ok(val) = serde_json::to_value(&settings) {
            store.set(STORE_KEY, val);
            let _ = store.save();
        }
    }

    app.emit("lastfm-status-changed", &status)
        .map_err(|e| e.to_string())?;
    app.emit(
        "lastfm-auth-polling",
        serde_json::json!({ "polling": false }),
    )
    .map_err(|e| e.to_string())?;

    info!("lastfm auth complete: username={}", session.username);
    Ok(())
}

// ---------------------------------------------------------------------------
// Tauri コマンド
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn lastfm_get_auth_token(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let client = state.0.lock().unwrap().lastfm_client.clone();

    if client.api_key.is_empty() {
        return Err(
            "LASTFM_API_KEY がビルド時に埋め込まれていません。環境変数を設定して再ビルドしてください"
                .to_string(),
        );
    }

    let token = client.get_token().await?;

    // 既存のポーリングをキャンセル
    if let Some(old) = state.0.lock().unwrap().auth_poll_cancel_token.take() {
        old.cancel();
    }

    // 一時トークンと新しいキャンセルトークンを AppState へ保存
    let cancel_token = CancellationToken::new();
    {
        let mut inner = state.0.lock().unwrap();
        inner.pending_auth_token = Some(token.clone());
        inner.auth_poll_cancel_token = Some(cancel_token.clone());
    }

    // ブラウザで認証 URL を開く
    let auth_url = format!(
        "https://www.last.fm/api/auth/?api_key={}&token={}",
        client.api_key, token
    );
    app.opener()
        .open_url(&auth_url, None::<&str>)
        .map_err(|e| e.to_string())?;

    // ポーリング開始を UI に通知
    app.emit(
        "lastfm-auth-polling",
        serde_json::json!({ "polling": true }),
    )
    .map_err(|e| e.to_string())?;

    // バックグラウンドでセッション取得をポーリング
    let app_clone = app.clone();
    let state_arc = Arc::clone(&state.0);
    tokio::spawn(async move {
        let client = {
            state_arc.lock().unwrap().lastfm_client.clone()
        };
        let deadline =
            tokio::time::Instant::now() + tokio::time::Duration::from_secs(AUTH_POLL_TIMEOUT_SECS);

        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    info!("lastfm auth polling cancelled");
                    break;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(AUTH_POLL_INTERVAL_SECS)) => {
                    if tokio::time::Instant::now() >= deadline {
                        warn!("lastfm auth polling timed out after {}s", AUTH_POLL_TIMEOUT_SECS);
                        break;
                    }

                    match client.get_session(&token).await {
                        Ok(session) => {
                            if let Err(e) = complete_auth(&app_clone, &state_arc, session).await {
                                warn!("lastfm complete_auth error: {e}");
                            }
                            return;
                        }
                        Err(e) if e.contains("not been authorised") || e.contains("14") => {
                            // ユーザーがまだブラウザで承認していない → 継続
                        }
                        Err(e) => {
                            // ネットワークエラー等 → 継続（致命的でなければ）
                            warn!("lastfm auth polling error (will retry): {e}");
                        }
                    }
                }
            }
        }

        // ループ終了後のクリーンアップ
        {
            let mut inner = state_arc.lock().unwrap();
            inner.auth_poll_cancel_token = None;
        }
        app_clone
            .emit(
                "lastfm-auth-polling",
                serde_json::json!({ "polling": false }),
            )
            .ok();
    });

    Ok(())
}

/// 手動フォールバック: ブラウザ承認後にユーザーが明示的に呼び出す場合
#[tauri::command]
pub async fn lastfm_get_session(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let token = state
        .0
        .lock()
        .unwrap()
        .pending_auth_token
        .clone()
        .ok_or_else(|| "先に [Last.fm でログイン] をクリックしてください".to_string())?;

    // ポーリングを止める
    if let Some(cancel) = state.0.lock().unwrap().auth_poll_cancel_token.take() {
        cancel.cancel();
    }

    let client = state.0.lock().unwrap().lastfm_client.clone();
    let session = client.get_session(&token).await?;

    complete_auth(&app, &state.0, session).await
}

/// 認証ポーリングをキャンセルする
#[tauri::command]
pub fn lastfm_cancel_auth(state: tauri::State<'_, AppState>) {
    let mut inner = state.0.lock().unwrap();
    if let Some(cancel) = inner.auth_poll_cancel_token.take() {
        cancel.cancel();
        inner.pending_auth_token = None;
        info!("lastfm auth polling cancelled by user");
    }
}

#[tauri::command]
pub async fn lastfm_logout(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // 認証ポーリング中なら停止
    if let Some(cancel) = state.0.lock().unwrap().auth_poll_cancel_token.take() {
        cancel.cancel();
    }

    // Last.fm ポーリング（Scrobbling）も停止
    let was_polling = {
        let mut inner = state.0.lock().unwrap();
        if let Some(token) = inner.poll_cancel_token.take() {
            token.cancel();
            true
        } else {
            false
        }
    };
    if was_polling {
        app.emit(
            "polling-status-changed",
            serde_json::json!({ "running": false }),
        )
        .map_err(|e| e.to_string())?;
    }

    // keyring から session_key を削除
    if let Ok(entry) = Entry::new(KEYRING_SERVICE, KEYRING_SESSION_KEY) {
        let _ = entry.delete_password();
    }

    {
        let mut inner = state.0.lock().unwrap();
        inner.pending_auth_token = None;
        inner.settings.lastfm_username = String::new();
        inner.auth_status = AuthStatus {
            authenticated: false,
            username: None,
        };
    }

    // username を Store からも削除
    if let Ok(store) = app.store(STORE_PATH) {
        if let Ok(mut settings) = store
            .get(STORE_KEY)
            .and_then(|v| serde_json::from_value::<crate::models::settings::Settings>(v).ok())
            .ok_or(())
        {
            settings.lastfm_username = String::new();
            if let Ok(val) = serde_json::to_value(&settings) {
                store.set(STORE_KEY, val);
                let _ = store.save();
            }
        }
    }

    // Discord Rich Presence をクリア
    {
        let discord_client = {
            let inner = state.0.lock().unwrap();
            std::sync::Arc::clone(&inner.discord_client)
        };
        let mut client = discord_client.lock().unwrap();
        if client.is_connected() {
            if let Err(e) = client.clear_activity() {
                warn!("clear_activity on logout: {e}");
            }
        }
    }

    app.emit(
        "lastfm-status-changed",
        AuthStatus {
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
