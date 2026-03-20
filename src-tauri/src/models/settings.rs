use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    // Last.fm
    pub lastfm_username: String,

    // Discord RPC
    pub discord_app_id: String,
    pub discord_enabled: bool,
    pub rpc_details_format: String,
    pub rpc_state_format: String,
    pub rpc_name_format: String,
    pub rpc_use_listening_type: bool,
    pub rpc_show_album_art: bool,
    pub rpc_show_timestamp: bool,
    pub rpc_show_lastfm_button: bool,

    // General
    pub poll_interval_secs: u64,
    pub start_on_login: bool,
    pub minimize_to_tray: bool,
    pub language: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            lastfm_username: String::new(),
            discord_app_id: String::new(),
            discord_enabled: true,
            rpc_details_format: "{artist} - {track}".to_string(),
            rpc_state_format: "{album}".to_string(),
            rpc_name_format: "{track}".to_string(),
            rpc_use_listening_type: true,
            rpc_show_album_art: true,
            rpc_show_timestamp: true,
            rpc_show_lastfm_button: true,
            poll_interval_secs: 15,
            start_on_login: false,
            minimize_to_tray: true,
            language: "ja".to_string(),
        }
    }
}
