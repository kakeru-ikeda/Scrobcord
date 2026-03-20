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
