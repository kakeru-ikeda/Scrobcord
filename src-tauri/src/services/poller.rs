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
        let mut no_track_ticks: u32 = 0;
        // AppState から共有クライアントを取得（reqwest コネクションプールを使い回す）
        let lastfm_client = state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .lastfm_client
            .clone();

        loop {
            tokio::select! {
                _ = child.cancelled() => {
                    info!("polling: cancelled");
                    break;
                }
                _ = poll_once(&app, &state, &lastfm_client, &mut prev_track, &mut no_track_ticks) => {}
            }

            let interval = {
                state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .settings
                    .poll_interval_secs
            };

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
            let discord_client = {
                let inner = state.lock().unwrap_or_else(|e| e.into_inner());
                Arc::clone(&inner.discord_client)
            };
            let mut client = discord_client.lock().unwrap_or_else(|e| e.into_inner());
            if client.is_connected() {
                if let Err(e) = client.clear_activity() {
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

/// null が何連続したらアクティビティをクリアするか
/// 15秒ポーリングでは曲切り替え時の一瞬の gap を踏む確率は非常に低いため 1 で十分。
/// ポーズ時は1ティック（最大15秒）でアクティビティがクリアされる。
const CLEAR_THRESHOLD: u32 = 1;

/// 1回のポーリング処理
async fn poll_once(
    app: &AppHandle,
    state: &Arc<Mutex<AppStateInner>>,
    client: &LastfmClient,
    prev_track: &mut Option<Track>,
    no_track_ticks: &mut u32,
) {
    let (configured_username, auth_username, authenticated) = {
        let inner = state.lock().unwrap_or_else(|e| e.into_inner());
        (
            inner.settings.lastfm_username.clone(),
            inner.auth_status.username.clone(),
            inner.auth_status.authenticated,
        )
    };

    let username = if configured_username.is_empty() {
        auth_username.unwrap_or_default()
    } else {
        configured_username
    };

    debug!(
        "polling: tick username='{}' authenticated={}",
        username, authenticated
    );

    if username.is_empty() {
        warn!("polling: lastfm username is empty, skip tick (authenticated={authenticated})");
        return;
    }

    let now_playing = match client.get_now_playing(&username).await {
        Ok(t) => t,
        Err(e) => {
            error!("polling last.fm: {e}");
            return;
        }
    };

    match now_playing.as_ref() {
        Some(track) => {
            debug!(
                "polling: detected now playing '{}' - '{}'",
                track.artist, track.title
            );
            *no_track_ticks = 0;
        }
        None => {
            *no_track_ticks += 1;
            debug!(
                "polling: no now-playing track returned for user '{}' (no_track_ticks={})",
                username, no_track_ticks
            );
            // CLEAR_THRESHOLD 以上の場合のみアクティビティをクリアする
            if *no_track_ticks < CLEAR_THRESHOLD {
                return;
            }
        }
    }

    let track_changed = !is_same_track(prev_track.as_ref(), now_playing.as_ref());

    if track_changed {
        // 状態更新
        {
            let mut inner = state.lock().unwrap_or_else(|e| e.into_inner());
            inner.now_playing = now_playing.clone();
        }

        // track-changed イベントを emit
        app.emit("track-changed", serde_json::json!({ "track": now_playing }))
            .unwrap_or_else(|e| warn!("emit track-changed: {e}"));

        *prev_track = now_playing.clone();
    }

    // Discord RPC 更新
    let settings = {
        state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .clone()
    };
    let state2 = Arc::clone(state);
    let app2 = app.clone();
    let track_owned = now_playing.clone();
    tokio::task::spawn_blocking(move || {
        update_discord(
            &app2,
            &state2,
            &settings,
            track_owned.as_ref(),
            track_changed,
        );
    })
    .await
    .unwrap_or_else(|e| warn!("update_discord spawn_blocking failed: {:?}", e));
}

/// Discord RPC の状態を更新する
/// - spawn_blocking から呼ばれるブロッキング関数
/// - AppStateInner の Mutex は短時間のみ保持し、重い I/O は discord_client 専用の Mutex で行う
fn update_discord(
    app: &AppHandle,
    state: &Arc<Mutex<AppStateInner>>,
    settings: &Settings,
    track: Option<&Track>,
    track_changed: bool,
) {
    // AppStateInner から discord_client の Arc だけを短時間で取り出す
    let discord_client = {
        let inner = state.lock().unwrap_or_else(|e| e.into_inner());
        Arc::clone(&inner.discord_client)
    };

    // discord_client の Mutex を保持して I/O 処理（AppStateInner の Mutex は解放済み）
    let mut client = discord_client.lock().unwrap_or_else(|e| e.into_inner());
    let mut newly_connected = false;
    let mut can_update_activity = true;
    let mut final_status: Option<DiscordStatus> = None;

    if !client.is_connected() {
        client.app_id = settings.discord_app_id.clone();
        match client.connect() {
            Ok(()) => {
                newly_connected = true;
                final_status = Some(DiscordStatus {
                    connected: true,
                    error: None,
                });
            }
            Err(e) => {
                warn!("discord connect: {e}");
                final_status = Some(DiscordStatus {
                    connected: false,
                    error: Some(e),
                });
                can_update_activity = false;
            }
        }
    }

    if can_update_activity && client.is_connected() && (track_changed || newly_connected) {
        if let Some(t) = track {
            if let Err(e) = client.set_activity(t, settings) {
                warn!("set_activity: {e}");
                client.disconnect();
                final_status = Some(DiscordStatus {
                    connected: false,
                    error: Some(e),
                });
            } else {
                info!("discord activity updated: '{}' - '{}'", t.artist, t.title);
            }
        } else if let Err(e) = client.clear_activity() {
            warn!("clear_activity: {e}");
            client.disconnect();
            final_status = Some(DiscordStatus {
                connected: false,
                error: Some(e),
            });
        } else {
            info!("discord activity cleared");
        }
    }

    // I/O 完了後に discord_client の Mutex を解放してから AppStateInner に書き戻す
    drop(client);

    if let Some(ref status) = final_status {
        state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .discord_status = status.clone();
    }

    if let Some(status) = final_status {
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

/// 設定変更後など、トラック変化を待たずに Discord アクティビティをすぐ更新したいときに呼ぶ。
/// AppStateInner から最新のトラックを取得して `update_discord` を強制実行する。
pub fn refresh_discord(app: &AppHandle, state: &Arc<Mutex<AppStateInner>>, settings: &Settings) {
    let now_playing = {
        state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .now_playing
            .clone()
    };
    update_discord(app, state, settings, now_playing.as_ref(), true);
}
