use std::sync::{Arc, Mutex};

use log::{debug, error, info, warn};
use tauri::{AppHandle, Emitter};
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;

use crate::models::{settings::Settings, status::DiscordStatus, track::Track};
use crate::services::lastfm::LastfmClient;
use crate::state::AppStateInner;

/// ポーリングタスクを tauri::async_runtime::spawn で起動し CancellationToken を返す
pub fn start(app: AppHandle, state: Arc<Mutex<AppStateInner>>) -> CancellationToken {
    let token = CancellationToken::new();
    let child = token.clone();

    tauri::async_runtime::spawn(async move {
        info!("polling: started");
        app.emit(
            "polling-status-changed",
            serde_json::json!({ "running": true }),
        )
        .unwrap_or_else(|e| warn!("emit polling-status-changed: {e}"));

        let mut prev_track: Option<Track> = None;

        loop {
            tokio::select! {
                _ = child.cancelled() => {
                    info!("polling: cancelled");
                    break;
                }
                _ = poll_once(&app, &state, &mut prev_track) => {}
            }

            let interval = { state.lock().unwrap().settings.poll_interval_secs };

            tokio::select! {
                _ = child.cancelled() => {
                    info!("polling: cancelled (sleep)");
                    break;
                }
                _ = sleep(Duration::from_secs(interval)) => {}
            }
        }

        // 停止時に Rich Presence をクリア
        {
            let mut inner = state.lock().unwrap();
            if inner.discord_client.is_connected() {
                if let Err(e) = inner.discord_client.clear_activity() {
                    warn!("clear_activity on stop: {e}");
                }
            }
        }

        app.emit(
            "polling-status-changed",
            serde_json::json!({ "running": false }),
        )
        .unwrap_or_else(|e| warn!("emit polling-status-changed: {e}"));
        info!("polling: stopped");
    });

    token
}

/// 1回のポーリング処理
async fn poll_once(
    app: &AppHandle,
    state: &Arc<Mutex<AppStateInner>>,
    prev_track: &mut Option<Track>,
) {
    let (configured_username, auth_username, authenticated, discord_enabled) = {
        let inner = state.lock().unwrap();
        (
            inner.settings.lastfm_username.clone(),
            inner.auth_status.username.clone(),
            inner.auth_status.authenticated,
            inner.settings.discord_enabled,
        )
    };

    let username = if configured_username.is_empty() {
        auth_username.unwrap_or_default()
    } else {
        configured_username
    };

    debug!(
        "polling: tick username='{}' authenticated={} discord_enabled={}",
        username,
        authenticated,
        discord_enabled
    );

    if username.is_empty() {
        warn!(
            "polling: lastfm username is empty, skip tick (authenticated={authenticated})"
        );
        return;
    }

    let client = LastfmClient::new();
    let now_playing = match client.get_now_playing(&username).await {
        Ok(t) => t,
        Err(e) => {
            error!("polling last.fm: {e}");
            return;
        }
    };

    match now_playing.as_ref() {
        Some(track) => debug!(
            "polling: detected now playing '{}' - '{}'",
            track.artist,
            track.title
        ),
        None => debug!("polling: no now-playing track returned for user '{}'", username),
    }

    // 前回と同じなら何もしない
    if is_same_track(prev_track.as_ref(), now_playing.as_ref()) {
        return;
    }

    // 状態更新
    {
        let mut inner = state.lock().unwrap();
        inner.now_playing = now_playing.clone();
    }

    // track-changed イベントを emit
    app.emit("track-changed", serde_json::json!({ "track": now_playing }))
        .unwrap_or_else(|e| warn!("emit track-changed: {e}"));

    // Discord RPC 更新
    if discord_enabled {
        let settings = { state.lock().unwrap().settings.clone() };
        update_discord(app, state, &settings, now_playing.as_ref());
    }

    *prev_track = now_playing;
}

/// Discord RPC の状態を更新する（同期 I/O なので mutex 保持中に実行）
fn update_discord(
    app: &AppHandle,
    state: &Arc<Mutex<AppStateInner>>,
    settings: &Settings,
    track: Option<&Track>,
) {
    let mut inner = state.lock().unwrap();
    let mut status_to_emit: Option<DiscordStatus> = None;
    let mut can_update_activity = true;

    // 未接続なら接続を試みる
    if !inner.discord_client.is_connected() {
        inner.discord_client.app_id = settings.discord_app_id.clone();
        match inner.discord_client.connect() {
            Ok(()) => {
                let status = DiscordStatus {
                    connected: true,
                    error: None,
                };
                inner.discord_status = status.clone();
                status_to_emit = Some(status);
            }
            Err(e) => {
                warn!("discord connect: {e}");
                let status = DiscordStatus {
                    connected: false,
                    error: Some(e),
                };
                inner.discord_status = status.clone();
                status_to_emit = Some(status);
                can_update_activity = false;
            }
        }
    }

    if can_update_activity && inner.discord_client.is_connected() {
        if let Some(t) = track {
            if let Err(e) = inner.discord_client.set_activity(t, settings) {
                warn!("set_activity: {e}");
                // ソケット切断の可能性があるのでリセット
                inner.discord_client.disconnect();
                let status = DiscordStatus {
                    connected: false,
                    error: Some(e),
                };
                inner.discord_status = status.clone();
                status_to_emit = Some(status);
            }
        } else if let Err(e) = inner.discord_client.clear_activity() {
            warn!("clear_activity: {e}");
            let status = DiscordStatus {
                connected: false,
                error: Some(e),
            };
            inner.discord_status = status.clone();
            status_to_emit = Some(status);
        }
    }

    drop(inner);

    if let Some(status) = status_to_emit {
        app.emit("discord-status-changed", &status)
            .unwrap_or_else(|e| warn!("emit discord-status-changed: {e}"));
    }
}

fn is_same_track(a: Option<&Track>, b: Option<&Track>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.title == b.title && a.artist == b.artist,
        _ => false,
    }
}
