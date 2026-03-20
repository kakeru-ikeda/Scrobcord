use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

use crate::models::{
    settings::Settings,
    status::{AuthStatus, DiscordStatus},
    track::Track,
};
use crate::services::discord_rpc::DiscordRpcClient;

pub struct AppStateInner {
    pub settings: Settings,
    pub auth_status: AuthStatus,
    pub discord_status: DiscordStatus,
    pub now_playing: Option<Track>,
    pub poll_cancel_token: Option<CancellationToken>,
    pub discord_client: DiscordRpcClient,
    /// Last.fm OAuth の一時トークン（getToken → getSession 間のみ保持）
    #[allow(dead_code)]
    pub pending_auth_token: Option<String>,
}

pub struct AppState(pub Arc<Mutex<AppStateInner>>);
