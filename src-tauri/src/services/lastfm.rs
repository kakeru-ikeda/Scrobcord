// Phase 2 で実装
pub struct LastfmClient {
    pub api_key: String,
    api_secret: String,
    client: reqwest::Client,
}

impl LastfmClient {
    pub fn new(api_key: String, api_secret: String) -> Self {
        Self {
            api_key,
            api_secret,
            client: reqwest::Client::new(),
        }
    }
}
