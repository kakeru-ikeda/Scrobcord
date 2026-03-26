mod commands;
mod models;
mod services;
mod state;

use models::settings::Settings;
use models::status::{AuthStatus, DiscordStatus};
use services::discord_rpc::DiscordRpcClient;
use services::lastfm::LastfmClient;
use state::{AppState, AppStateInner};
use std::sync::{Arc, Mutex};
use tauri::Manager;

/// 言語コードに応じたトレイメニューのラベルを返す (show, toggle, quit)
fn tray_labels(lang: &str) -> (&'static str, &'static str, &'static str) {
    match lang {
        "ja" => ("表示", "停止 / 再開", "終了"),
        _ => ("Show", "Pause / Resume", "Quit"),
    }
}

/// トレイメニューのラベルを現在の言語で更新する
pub fn update_tray_labels(app: &tauri::AppHandle, lang: &str) {
    use tauri::menu::{Menu, MenuItem};
    let (show, toggle, quit) = tray_labels(lang);
    if let Some(tray) = app.tray_by_id("main") {
        if let (Ok(s), Ok(t), Ok(q)) = (
            MenuItem::with_id(app, "show", show, true, None::<&str>),
            MenuItem::with_id(app, "toggle", toggle, true, None::<&str>),
            MenuItem::with_id(app, "quit", quit, true, None::<&str>),
        ) {
            if let Ok(menu) = Menu::with_items(app, &[&s, &t, &q]) {
                tray.set_menu(Some(menu)).ok();
            }
        }
    }
}

/// Discord アクティビティをクリアして切断する（終了処理の共通実装）
fn discord_cleanup(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let discord_client = {
        let inner = state.0.lock().unwrap();
        Arc::clone(&inner.discord_client)
    };
    let mut client = discord_client.lock().unwrap();
    if client.is_connected() {
        let _ = client.clear_activity();
        client.disconnect();
    }
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState(Arc::new(Mutex::new(AppStateInner {
        settings: Settings::default(),
        auth_status: AuthStatus {
            authenticated: false,
            username: None,
        },
        discord_status: DiscordStatus {
            connected: false,
            error: None,
        },
        now_playing: None,
        poll_cancel_token: None,
        discord_client: Arc::new(Mutex::new(DiscordRpcClient::new(String::new()))),
        pending_auth_token: None,
        auth_poll_cancel_token: None,
        lastfm_client: LastfmClient::new(),
    })));

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Debug)
                .build(),
        )
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                w.show().ok();
                w.unminimize().ok();
                w.set_focus().ok();
            }
        }))
        .manage(app_state)
        .setup(|app| {
            setup_app(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    let minimize = {
                        let state = window.state::<AppState>();
                        let guard = state.0.lock().unwrap();
                        guard.settings.minimize_to_tray
                    };
                    if minimize {
                        api.prevent_close();
                        window.hide().ok();
                    } else {
                        // トレイ最小化なし: ウィンドウを閉じる = 終了なのでアクティビティをクリア
                        discord_cleanup(window.app_handle());
                    }
                }
                tauri::WindowEvent::Destroyed => {
                    // どのケースでウィンドウが破棄されてもアクティビティをクリア
                    discord_cleanup(window.app_handle());
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth::lastfm_get_auth_token,
            commands::auth::lastfm_get_session,
            commands::auth::lastfm_cancel_auth,
            commands::auth::lastfm_logout,
            commands::auth::lastfm_get_auth_status,
            commands::discord::discord_connect,
            commands::discord::discord_disconnect,
            commands::discord::discord_get_status,
            commands::polling::start_polling,
            commands::polling::stop_polling,
            commands::polling::get_now_playing,
            commands::polling::get_polling_status,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::reset_saved_data,
            commands::history::get_recent_tracks,
            commands::updater::check_for_updates,
            commands::updater::open_release_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn setup_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::TrayIconBuilder;
    use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;

    // ストアから設定を読み込み AppState に反映
    let loaded = commands::settings::load_settings_from_store(app.handle());
    let start_on_login = loaded.start_on_login;
    let start_minimized = loaded.start_minimized;
    let language = loaded.language.clone();

    {
        let state = app.state::<AppState>();
        state.0.lock().unwrap().settings = loaded;
    }

    // autostart の有効/無効を設定と同期
    let autostart = app.autolaunch();
    if start_on_login {
        autostart.enable().ok();
    } else {
        autostart.disable().ok();
    }

    // --autostart 引数付きで起動かつ start_minimized が有効な場合はウィンドウを非表示にする
    let is_autostart = std::env::args().any(|a| a == "--autostart");
    if is_autostart && start_minimized {
        if let Some(w) = app.get_webview_window("main") {
            w.hide().ok();
        }
    }

    // トレイメニューを構築
    let (show_label, toggle_label, quit_label) = tray_labels(&language);
    let show_item = MenuItem::with_id(app, "show", show_label, true, None::<&str>)?;
    let toggle_item = MenuItem::with_id(app, "toggle", toggle_label, true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", quit_label, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &toggle_item, &quit_item])?;

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .tooltip("Scrobcord")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    w.show().ok();
                    w.set_focus().ok();
                }
            }
            "toggle" => {
                let state = app.state::<AppState>();
                let running = state.0.lock().unwrap().poll_cancel_token.is_some();
                if running {
                    // 停止
                    if let Some(token) = state.0.lock().unwrap().poll_cancel_token.take() {
                        token.cancel();
                    }
                } else {
                    // 再開
                    let arc = Arc::clone(&state.0);
                    let token = crate::services::poller::start(app.clone(), arc);
                    state.0.lock().unwrap().poll_cancel_token = Some(token);
                }
            }
            "quit" => {
                // アクティビティとポーリングを停止してから終了
                discord_cleanup(app);
                if let Some(token) = app
                    .state::<AppState>()
                    .0
                    .lock()
                    .unwrap()
                    .poll_cancel_token
                    .take()
                {
                    token.cancel();
                }
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // 左クリックでウィンドウを表示/非表示トグル
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    if w.is_visible().unwrap_or(false) {
                        w.hide().ok();
                    } else {
                        w.show().ok();
                        w.set_focus().ok();
                    }
                }
            }
        })
        .build(app)?;

    // 設定読み込み後に自動ポーリング開始
    let arc = Arc::clone(&app.state::<AppState>().0);
    let handle = app.handle().clone();
    let token = crate::services::poller::start(handle, arc);
    app.state::<AppState>().0.lock().unwrap().poll_cancel_token = Some(token);

    Ok(())
}
