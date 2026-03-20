use std::collections::BTreeMap;

use crate::models::track::Track;

const API_ROOT: &str = "https://ws.audioscrobbler.com/2.0/";

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
    pub async fn get_session(&self, token: &str) -> Result<String, String> {
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

        resp["session"]["key"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| format!("session key not found: {resp}"))
    }

    // -----------------------------------------------------------------------
    // user.getRecentTracks (limit=1) → 現在再生中トラックを返す
    // -----------------------------------------------------------------------
    pub async fn get_now_playing(&self, username: &str) -> Result<Option<Track>, String> {
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
            .or_else(|| resp["recenttracks"]["track"].as_object().map(|_| &resp["recenttracks"]["track"]));

        let track_val = match tracks {
            Some(t) => t,
            None => return Ok(None),
        };

        // nowplaying 属性があるもののみ対象
        let is_nowplaying = track_val["@attr"]["nowplaying"]
            .as_str()
            .map(|s| s == "true")
            .unwrap_or(false);

        if !is_nowplaying {
            return Ok(None);
        }

        let title = track_val["name"]
            .as_str()
            .unwrap_or_default()
            .to_string();
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

        Ok(Some(Track {
            title,
            artist,
            album,
            album_art_url,
            url,
            timestamp: None,
        }))
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
        // Last.fm 公式ドキュメントの例に基づく検証
        // params: api_key=xxx, method=auth.getToken, secret=yyy
        // 期待値: MD5("api_keyxxxmethodauth.getTokenyyy")
        let client = LastfmClient::new("xxx".to_string(), "yyy".to_string());
        let mut params: BTreeMap<&str, String> = BTreeMap::new();
        params.insert("api_key", "xxx".to_string());
        params.insert("method", "auth.getToken".to_string());

        let sig = client.sign(&params);

        // python: hashlib.md5(b"api_keyxxxmethodauth.getTokenyyy").hexdigest()
        assert_eq!(sig, format!("{:x}", md5::compute(b"api_keyxxxmethodauth.getTokenyyy")));
    }

    #[test]
    fn test_sign_keys_sorted_lexicographically() {
        let client = LastfmClient::new("mykey".to_string(), "mysecret".to_string());
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
            md5::compute("api_keymykeymethod auth.getSessiontokenmytokenmysecret".replace(' ', "").as_bytes())
        );
        let manually = format!(
            "{:x}",
            md5::compute(
                format!("api_keymykeymethod{}tokenmytokenmysecret", "auth.getSession").as_bytes()
            )
        );
        assert_eq!(sig, manually);
        let _ = (expected, expected_correct); // suppress warnings
    }
}
