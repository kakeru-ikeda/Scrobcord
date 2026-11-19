use std::{collections::BTreeMap, time::Duration};

use log::debug;

use crate::models::track::{RecentTracksPage, ScrobbledTrack, Track};
use crate::services::{
    auth_flow::{self, AuthError, AuthToken},
    scrobsign::{self, SignError, Signature},
};

const API_ROOT: &str = "https://ws.audioscrobbler.com/2.0/";

// ビルド時に環境変数 LASTFM_API_KEY(新しいキーの api_key。秘密ではない)を埋め込む
const EMBEDDED_API_KEY: &str = match option_env!("LASTFM_API_KEY") {
    Some(k) => k,
    None => "",
};

#[derive(Clone)]
pub struct LastfmClient {
    pub api_key: String,
    client: reqwest::Client,
}

pub struct LastfmSession {
    pub key: String,
    pub username: String,
    #[allow(dead_code)] // Task 3 で外す
    pub api_key: String,
}

impl LastfmClient {
    pub fn new() -> Self {
        Self {
            api_key: EMBEDDED_API_KEY.to_string(),
            client: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .user_agent(concat!("Scrobcord/", env!("CARGO_PKG_VERSION")))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    pub async fn request_signature(
        &self,
        method: &str,
        params: &BTreeMap<String, String>,
    ) -> Result<Signature, SignError> {
        scrobsign::request_signature(&self.client, method, params).await
    }

    pub fn embedded_api_key(&self) -> &str {
        &self.api_key
    }

    // -----------------------------------------------------------------------
    // auth.getToken
    // -----------------------------------------------------------------------
    pub async fn get_token(&self) -> Result<AuthToken, AuthError> {
        for attempt in 0..=1 {
            let sig = self
                .request_signature("auth.getToken", &BTreeMap::new())
                .await
                .map_err(AuthError::Sign)?;
            auth_flow::check_signer_api_key(&sig.api_key, &self.api_key)?;
            let response = self
                .client
                .get(API_ROOT)
                .query(&[
                    ("method", "auth.getToken"),
                    ("api_key", sig.api_key.as_str()),
                    ("api_sig", sig.api_sig.as_str()),
                    ("format", "json"),
                ])
                .send()
                .await
                .map_err(|e| AuthError::Transport(e.without_url().to_string()))?;
            let body = response
                .text()
                .await
                .map_err(|e| AuthError::Transport(e.without_url().to_string()))?;
            match auth_flow::parse_token_response(&body) {
                Ok(token) => {
                    return Ok(AuthToken {
                        token,
                        api_key: sig.api_key,
                    })
                }
                Err(e) if auth_flow::should_resign_token_request(&e, attempt) => continue,
                Err(e) => return Err(e),
            }
        }
        unreachable!("token signature retry loop returns on its second attempt")
    }

    // -----------------------------------------------------------------------
    // auth.getSession
    // -----------------------------------------------------------------------
    pub async fn get_session_signed(
        &self,
        token: &str,
        sig: &Signature,
    ) -> Result<LastfmSession, AuthError> {
        let response = self
            .client
            .get(API_ROOT)
            .query(&[
                ("method", "auth.getSession"),
                ("api_key", sig.api_key.as_str()),
                ("token", token),
                ("api_sig", sig.api_sig.as_str()),
                ("format", "json"),
            ])
            .send()
            .await
            .map_err(|e| AuthError::Transport(e.without_url().to_string()))?;
        let body = response
            .text()
            .await
            .map_err(|e| AuthError::Transport(e.without_url().to_string()))?;
        let (key, username) = auth_flow::parse_session_response(&body)?;
        Ok(LastfmSession {
            key,
            username,
            api_key: sig.api_key.clone(),
        })
    }

    // -----------------------------------------------------------------------
    // user.getRecentTracks (limit=1) → 現在再生中トラックを返す
    // -----------------------------------------------------------------------
    pub async fn get_now_playing(&self, username: &str) -> Result<Option<Track>, String> {
        debug!("lastfm: requesting recent tracks for user='{}'", username);

        let response = self
            .client
            .get(API_ROOT)
            .query(&[
                ("method", "user.getRecentTracks"),
                ("user", username),
                ("api_key", &self.api_key),
                ("limit", "1"),
                ("format", "json"),
            ])
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| format!("failed to read response: {e}"))?;

        if status.is_server_error() || status.is_client_error() {
            return Err(format!("transient: Last.fm HTTP {} error", status.as_u16()));
        }

        let resp: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
            format!(
                "Last.fm returned non-JSON ({e}): {}",
                &body[..body.len().min(300)]
            )
        })?;

        if let Some(err_msg) = resp["message"].as_str() {
            return Err(format!("Last.fm error: {err_msg}"));
        }

        let tracks = resp["recenttracks"]["track"]
            .as_array()
            .and_then(|arr| arr.first())
            .or_else(|| {
                resp["recenttracks"]["track"]
                    .as_object()
                    .map(|_| &resp["recenttracks"]["track"])
            });

        let track_val = match tracks {
            Some(t) => t,
            None => {
                debug!(
                    "lastfm: recenttracks.track not found in response for user='{}'",
                    username
                );
                return Ok(None);
            }
        };

        // nowplaying 属性があるもののみ対象
        let is_nowplaying = track_val["@attr"]["nowplaying"]
            .as_str()
            .map(|s| s == "true")
            .or_else(|| track_val["@attr"]["nowplaying"].as_bool())
            .unwrap_or(false);

        debug!(
            "lastfm: nowplaying_attr={:?} parsed_nowplaying={}",
            track_val["@attr"]["nowplaying"], is_nowplaying
        );

        if !is_nowplaying {
            debug!("lastfm: latest track is not marked as now playing");
            return Ok(None);
        }

        let title = track_val["name"].as_str().unwrap_or_default().to_string();
        let artist = track_val["artist"]["#text"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let album = track_val["album"]["#text"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        // extralarge 画像を優先
        let album_art_url = track_val["image"]
            .as_array()
            .and_then(|imgs| {
                imgs.iter()
                    .find(|img| img["size"].as_str() == Some("extralarge"))
                    .or_else(|| imgs.last())
            })
            .and_then(|img| img["#text"].as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let url = track_val["url"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        debug!("lastfm: parsed now playing '{}' - '{}'", artist, title);

        Ok(Some(Track {
            title,
            artist,
            album,
            album_art_url,
            url,
            timestamp: None,
        }))
    }

    // -----------------------------------------------------------------------
    // user.getRecentTracks — ページネーション付き再生履歴取得
    // -----------------------------------------------------------------------
    pub async fn get_recent_tracks(
        &self,
        username: &str,
        page: u32,
        limit: u32,
    ) -> Result<RecentTracksPage, String> {
        debug!(
            "lastfm: get_recent_tracks user='{}' page={} limit={}",
            username, page, limit
        );

        let body = self
            .client
            .get(API_ROOT)
            .query(&[
                ("method", "user.getRecentTracks"),
                ("user", username),
                ("api_key", &self.api_key),
                ("page", &page.to_string()),
                ("limit", &limit.to_string()),
                ("format", "json"),
            ])
            .send()
            .await
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| format!("failed to read response: {e}"))?;
        let resp: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
            format!(
                "Last.fm returned non-JSON ({e}): {}",
                &body[..body.len().min(300)]
            )
        })?;

        if let Some(err_msg) = resp["message"].as_str() {
            return Err(format!("Last.fm error: {err_msg}"));
        }

        let recent = &resp["recenttracks"];

        // ページネーション情報（API は文字列で返す）
        let parse_u32 = |key: &str| -> u32 {
            recent["@attr"][key]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0)
        };
        let total_tracks: u64 = recent["@attr"]["total"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let page_num = parse_u32("page");
        let per_page = parse_u32("perPage");
        let total_pages = parse_u32("totalPages");

        // track は配列/単一オブジェクト両対応
        let track_values: Vec<&serde_json::Value> = match &recent["track"] {
            serde_json::Value::Array(arr) => arr.iter().collect(),
            obj @ serde_json::Value::Object(_) => vec![obj],
            _ => vec![],
        };

        let mut tracks = Vec::with_capacity(track_values.len());
        for t in track_values {
            let now_playing = t["@attr"]["nowplaying"]
                .as_str()
                .map(|s| s == "true")
                .unwrap_or(false);

            let title = t["name"].as_str().unwrap_or_default().to_string();
            let artist = t["artist"]["#text"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let album = t["album"]["#text"].as_str().unwrap_or_default().to_string();

            let album_art_url = t["image"]
                .as_array()
                .and_then(|imgs| {
                    imgs.iter()
                        .find(|img| img["size"].as_str() == Some("extralarge"))
                        .or_else(|| imgs.last())
                })
                .and_then(|img| img["#text"].as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());

            let url = t["url"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());

            let timestamp: Option<i64> = if now_playing {
                None
            } else {
                t["date"]["uts"].as_str().and_then(|s| s.parse().ok())
            };

            tracks.push(ScrobbledTrack {
                title,
                artist,
                album,
                album_art_url,
                url,
                timestamp,
                now_playing,
            });
        }

        debug!(
            "lastfm: got {} tracks (page {}/{}) total={}",
            tracks.len(),
            page_num,
            total_pages,
            total_tracks
        );

        Ok(RecentTracksPage {
            tracks,
            page: page_num,
            per_page,
            total_pages,
            total_tracks,
        })
    }
}
