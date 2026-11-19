use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

use crate::models::{
    settings::Settings,
    status::{AuthStatus, DiscordStatus},
    track::Track,
};
use crate::services::discord_rpc::DiscordRpcClient;
use crate::services::lastfm::LastfmClient;
use crate::services::scrobsign::Signature;

pub struct PendingAuth {
    pub token: String,
    pub signature: Option<Signature>,
}

pub struct AppStateInner {
    pub settings: Settings,
    pub auth_status: AuthStatus,
    pub discord_status: DiscordStatus,
    pub now_playing: Option<Track>,
    pub poll_cancel_token: Option<CancellationToken>,
    /// Discord クライアントは独立した Mutex で管理する。
    /// AppStateInner の Mutex を保持したままブロッキング I/O を行うと
    /// tokio ワーカースレッドが詰まって UI が「応答なし」になるため。
    pub discord_client: Arc<Mutex<DiscordRpcClient>>,
    /// Last.fm OAuth の一時トークン（getToken → getSession 間のみ保持）
    pub pending_auth: Option<PendingAuth>,
    /// Last.fm 認証ポーリングタスクのキャンセルトークン
    pub auth_poll_cancel_token: Option<CancellationToken>,
    /// 共有 Last.fm クライアント（reqwest コネクションプールを使い回す）
    pub lastfm_client: LastfmClient,
}

pub struct AppState(pub Arc<Mutex<AppStateInner>>);
