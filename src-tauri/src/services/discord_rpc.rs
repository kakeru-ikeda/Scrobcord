// Phase 3 で実装
pub struct DiscordRpcClient {
    pub app_id: String,
}

impl DiscordRpcClient {
    pub fn new(app_id: String) -> Self {
        Self { app_id }
    }
}
