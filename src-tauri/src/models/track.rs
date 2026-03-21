use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_art_url: Option<String>,
    pub url: Option<String>,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrobbledTrack {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_art_url: Option<String>,
    pub url: Option<String>,
    /// UNIX 秒（nowplaying 時は None）
    pub timestamp: Option<i64>,
    pub now_playing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentTracksPage {
    pub tracks: Vec<ScrobbledTrack>,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
    pub total_tracks: u64,
}
