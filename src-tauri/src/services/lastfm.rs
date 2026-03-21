use std::collections::BTreeMap;

use log::debug;

use crate::models::track::{RecentTracksPage, ScrobbledTrack, Track};

const API_ROOT: &str = "https://ws.audioscrobbler.com/2.0/";

// ビルド時に環境変数 LASTFM_API_KEY / LASTFM_API_SECRET を埋め込む
const EMBEDDED_API_KEY: &str = match option_env!("LASTFM_API_KEY") {
    Some(k) => k,
    None => "",
};
const EMBEDDED_API_SECRET: &str = match option_env!("LASTFM_API_SECRET") {
    Some(k) => k,
    None => "",
};

#[derive(Clone)]
pub struct LastfmClient {
    pub api_key: String,
    api_secret: String,
    client: reqwest::Client,
}

pub struct LastfmSession {
    pub key: String,
    pub username: String,
}

impl LastfmClient {
    pub fn new() -> Self {
        Self {
            api_key: EMBEDDED_API_KEY.to_string(),
            api_secret: EMBEDDED_API_SECRET.to_string(),
            client: reqwest::Client::new(),
        }
    }

    // -----------------------------------------------------------------------
    // 署名生成: MD5(キー昇順連結 + api_secret)
    // -----------------------------------------------------------------------
    fn sign(&self, params: &BTreeMap<&str, String>) -> String {
        let mut plain = String::new();
        for (k, v) in params {
            plain.push_str(k);
            plain.push_str(v);
        }
        plain.push_str(&self.api_secret);
        format!("{:x}", md5::compute(plain.as_bytes()))
    }

    // -----------------------------------------------------------------------
    // auth.getToken
    // -----------------------------------------------------------------------
    pub async fn get_token(&self) -> Result<String, String> {
        let mut params: BTreeMap<&str, String> = BTreeMap::new();
        params.insert("api_key", self.api_key.clone());
        params.insert("method", "auth.getToken".to_string());
        let api_sig = self.sign(&params);

        let resp = self
            .client
            .get(API_ROOT)
            .query(&[
                ("method", "auth.getToken"),
                ("api_key", &self.api_key),
                ("api_sig", &api_sig),
                ("format", "json"),
            ])
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<serde_json::Value>()
            .await
            .map_err(|e| e.to_string())?;

        resp["token"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| format!("token not found: {resp}"))
    }

    // -----------------------------------------------------------------------
    // auth.getSession
    // -----------------------------------------------------------------------
    pub async fn get_session(&self, token: &str) -> Result<LastfmSession, String> {
        let mut params: BTreeMap<&str, String> = BTreeMap::new();
        params.insert("api_key", self.api_key.clone());
        params.insert("method", "auth.getSession".to_string());
        params.insert("token", token.to_string());
        let api_sig = self.sign(&params);

        let resp = self
            .client
            .get(API_ROOT)
            .query(&[
                ("method", "auth.getSession"),
                ("api_key", &self.api_key),
                ("token", token),
                ("api_sig", &api_sig),
                ("format", "json"),
            ])
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<serde_json::Value>()
            .await
            .map_err(|e| e.to_string())?;

        // エラーレスポンス確認
        if let Some(err_msg) = resp["message"].as_str() {
            return Err(format!("Last.fm error: {err_msg}"));
        }

        let key = resp["session"]["key"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| format!("session key not found: {resp}"))?;

        let username = resp["session"]["name"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| format!("session username not found: {resp}"))?;

        Ok(LastfmSession { key, username })
    }

    // -----------------------------------------------------------------------
    // user.getRecentTracks (limit=1) → 現在再生中トラックを返す
    // -----------------------------------------------------------------------
    pub async fn get_now_playing(&self, username: &str) -> Result<Option<Track>, String> {
        debug!("lastfm: requesting recent tracks for user='{}'", username);

        let resp = self
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
            .map_err(|e| e.to_string())?
            .json::<serde_json::Value>()
            .await
            .map_err(|e| e.to_string())?;

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

        let resp = self
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
            .json::<serde_json::Value>()
            .await
            .map_err(|e| e.to_string())?;

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_correct_order_and_hash() {
        // params: api_key=xxx, method=auth.getToken, secret=yyy のテスト用に直接インスタンス生成
        let client = LastfmClient {
            api_key: "xxx".to_string(),
            api_secret: "yyy".to_string(),
            client: reqwest::Client::new(),
        };
        let mut params: BTreeMap<&str, String> = BTreeMap::new();
        params.insert("api_key", "xxx".to_string());
        params.insert("method", "auth.getToken".to_string());

        let sig = client.sign(&params);

        // python: hashlib.md5(b"api_keyxxxmethodauth.getTokenyyy").hexdigest()
        assert_eq!(
            sig,
            format!("{:x}", md5::compute(b"api_keyxxxmethodauth.getTokenyyy"))
        );
    }

    #[test]
    fn test_sign_keys_sorted_lexicographically() {
        let client = LastfmClient {
            api_key: "mykey".to_string(),
            api_secret: "mysecret".to_string(),
            client: reqwest::Client::new(),
        };
        let mut params: BTreeMap<&str, String> = BTreeMap::new();
        params.insert("token", "mytoken".to_string());
        params.insert("api_key", "mykey".to_string());
        params.insert("method", "auth.getSession".to_string());

        let sig = client.sign(&params);
        // BTreeMap はキー昇順: api_key, method, token
        let expected = format!(
            "{:x}",
            md5::compute(b"api_keymykeyauthmethod.getSessiontokenmytokenmysecret")
        );
        // 正しい連結: "api_key" + "mykey" + "method" + "auth.getSession" + "token" + "mytoken" + "mysecret"
        let expected_correct = format!(
            "{:x}",
            md5::compute(
                "api_keymykeymethod auth.getSessiontokenmytokenmysecret"
                    .replace(' ', "")
                    .as_bytes()
            )
        );
        let manually = format!(
            "{:x}",
            md5::compute(
                format!(
                    "api_keymykeymethod{}tokenmytokenmysecret",
                    "auth.getSession"
                )
                .as_bytes()
            )
        );
        assert_eq!(sig, manually);
        let _ = (expected, expected_correct); // suppress warnings
    }
}
