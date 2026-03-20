mod commands;
mod models;
mod services;
mod state;

use models::settings::Settings;
use models::status::{AuthStatus, DiscordStatus};
use services::discord_rpc::DiscordRpcClient;
use state::{AppState, AppStateInner};
use std::sync::{Arc, Mutex};

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
        discord_client: DiscordRpcClient::new(String::new()),
    })));

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Debug)
                .build(),
        )
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::auth::lastfm_get_auth_token,
            commands::auth::lastfm_get_session,
            commands::auth::lastfm_logout,
            commands::auth::lastfm_get_auth_status,
            commands::discord::discord_connect,
            commands::discord::discord_disconnect,
            commands::discord::discord_get_status,
            commands::polling::start_polling,
            commands::polling::stop_polling,
            commands::polling::get_now_playing,
            commands::settings::get_settings,
            commands::settings::save_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
