use super::scrobsign::{SignError, Signature};

pub const LFM_INVALID_TOKEN: i64 = 4;
pub const LFM_INVALID_API_KEY: i64 = 10;
pub const LFM_INVALID_SIGNATURE: i64 = 13;
pub const LFM_UNAUTHORIZED_TOKEN: i64 = 14;
pub const LFM_TOKEN_EXPIRED: i64 = 15;
pub const LFM_SUSPENDED_API_KEY: i64 = 26;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastfmApiError {
    pub code: i64,
    pub message: String,
}

pub fn parse_lastfm_error(body: &serde_json::Value) -> Option<LastfmApiError> {
    let code = body["error"]
        .as_i64()
        .or_else(|| body["error"].as_str()?.parse().ok())?;
    Some(LastfmApiError {
        code,
        message: body["message"].as_str().unwrap_or_default().to_owned(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthToken {
    pub token: String,
    pub api_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    Sign(SignError),
    Lastfm(LastfmApiError),
    Transport(String),
    BadResponse(String),
    KeyMismatch,
}

impl AuthError {
    pub fn user_message(&self) -> String {
        match self {
            Self::Sign(e) => e.user_message(),
            Self::Lastfm(e) if matches!(e.code, LFM_INVALID_API_KEY | LFM_SUSPENDED_API_KEY) => format!("Last.fm のサービス側の問題で認証できません（error {}）", e.code),
            Self::Lastfm(e) if e.code == LFM_INVALID_SIGNATURE => "Last.fm の署名が無効です（error 13）。しばらくしてから再度お試しください".into(),
            Self::Lastfm(e) if matches!(e.code, LFM_INVALID_TOKEN | LFM_TOKEN_EXPIRED) => "認証トークンが無効か期限切れです。もう一度ログインしてください".into(),
            Self::Lastfm(e) => format!("Last.fm エラー {}: {}", e.code, e.message),
            Self::Transport(d) => format!("Last.fm に接続できません: {d}"),
            Self::BadResponse(d) => format!("Last.fm の応答を解釈できません（{d}）"),
            Self::KeyMismatch => "署名サービスの api_key とアプリに埋め込まれた api_key が一致しません。ビルド設定（LASTFM_API_KEY）を確認してください".into(),
        }
    }
}
impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.user_message())
    }
}

pub fn parse_token_response(body: &str) -> Result<String, AuthError> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|_| AuthError::BadResponse(format!("non-JSON body ({} bytes)", body.len())))?;
    if let Some(e) = parse_lastfm_error(&value) {
        return Err(AuthError::Lastfm(e));
    }
    value["token"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| AuthError::BadResponse("token missing".into()))
}

pub fn parse_session_response(body: &str) -> Result<(String, String), AuthError> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|_| AuthError::BadResponse(format!("non-JSON body ({} bytes)", body.len())))?;
    if let Some(e) = parse_lastfm_error(&value) {
        return Err(AuthError::Lastfm(e));
    }
    let key = value["session"]["key"]
        .as_str()
        .ok_or_else(|| AuthError::BadResponse("session key missing".into()))?;
    let name = value["session"]["name"]
        .as_str()
        .ok_or_else(|| AuthError::BadResponse("session name missing".into()))?;
    Ok((key.into(), name.into()))
}

pub fn should_resign_token_request(err: &AuthError, attempt: u32) -> bool {
    attempt == 0
        && matches!(
            err,
            AuthError::Lastfm(LastfmApiError {
                code: LFM_INVALID_SIGNATURE,
                ..
            })
        )
}
pub fn check_signer_api_key(signer_api_key: &str, embedded_api_key: &str) -> Result<(), AuthError> {
    if signer_api_key == embedded_api_key {
        Ok(())
    } else {
        Err(AuthError::KeyMismatch)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PollAction {
    Retry,
    RetryWithFreshSignature,
    Stop(String),
}

#[derive(Debug, Default)]
pub struct TokenSignatureCache {
    signature: Option<Signature>,
    resigned: bool,
}
impl TokenSignatureCache {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn get(&self) -> Option<&Signature> {
        self.signature.as_ref()
    }
    pub fn store(&mut self, sig: Signature) {
        self.signature = Some(sig);
    }
    pub fn on_error(&mut self, err: &AuthError) -> PollAction {
        match err {
            AuthError::Lastfm(e) if e.code == LFM_UNAUTHORIZED_TOKEN => PollAction::Retry,
            AuthError::Lastfm(e) if e.code == LFM_INVALID_SIGNATURE && !self.resigned => {
                self.resigned = true;
                self.signature = None;
                PollAction::RetryWithFreshSignature
            }
            AuthError::Lastfm(e) if e.code == LFM_INVALID_SIGNATURE => {
                PollAction::Stop(err.user_message())
            }
            AuthError::Lastfm(e)
                if matches!(
                    e.code,
                    LFM_INVALID_API_KEY
                        | LFM_SUSPENDED_API_KEY
                        | LFM_INVALID_TOKEN
                        | LFM_TOKEN_EXPIRED
                ) =>
            {
                PollAction::Stop(err.user_message())
            }
            AuthError::Lastfm(_) | AuthError::Transport(_) | AuthError::BadResponse(_) => {
                PollAction::Retry
            }
            AuthError::Sign(e) if e.is_retryable() => PollAction::Retry,
            AuthError::Sign(_) | AuthError::KeyMismatch => PollAction::Stop(err.user_message()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoredSessionState {
    Valid,
    NotLoggedIn,
    KeyChanged,
}
pub fn evaluate_stored_session(
    has_session_key: bool,
    stored_api_key: Option<&str>,
    embedded_api_key: &str,
) -> StoredSessionState {
    if !has_session_key {
        StoredSessionState::NotLoggedIn
    } else if !embedded_api_key.is_empty() && stored_api_key == Some(embedded_api_key) {
        StoredSessionState::Valid
    } else {
        StoredSessionState::KeyChanged
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn lf(code: i64) -> AuthError {
        AuthError::Lastfm(LastfmApiError {
            code,
            message: "dummy".into(),
        })
    }
    #[test]
    fn parses_api_errors_and_auth_bodies() {
        assert_eq!(
            parse_lastfm_error(&serde_json::json!({"error":14,"message":"x"}))
                .unwrap()
                .code,
            14
        );
        assert_eq!(
            parse_lastfm_error(&serde_json::json!({"error":"13"}))
                .unwrap()
                .code,
            13
        );
        assert!(parse_lastfm_error(&serde_json::json!({})).is_none());
        assert_eq!(
            parse_token_response(r#"{"token":"dummytoken"}"#).unwrap(),
            "dummytoken"
        );
        assert!(matches!(
            parse_token_response(r#"{"error":13,"message":"bad"}"#),
            Err(AuthError::Lastfm(_))
        ));
        let e = parse_token_response("private response").unwrap_err();
        assert!(!e.to_string().contains("private response"));
        assert_eq!(
            parse_session_response(r#"{"session":{"key":"dummysessionkey","name":"user"}}"#)
                .unwrap(),
            ("dummysessionkey".into(), "user".into())
        );
        assert!(matches!(
            parse_session_response(r#"{"error":14}"#),
            Err(AuthError::Lastfm(_))
        ));
        assert!(matches!(
            parse_session_response(r#"{"session":{"key":"dummysessionkey"}}"#),
            Err(AuthError::BadResponse(_))
        ));
    }
    #[test]
    fn resign_and_api_key_checks() {
        assert!(should_resign_token_request(&lf(13), 0));
        assert!(!should_resign_token_request(&lf(13), 1));
        assert!(!should_resign_token_request(&lf(14), 0));
        assert!(check_signer_api_key("a", "a").is_ok());
        assert_eq!(check_signer_api_key("a", "b"), Err(AuthError::KeyMismatch));
    }
    #[test]
    fn session_state_covers_legacy_and_mismatch() {
        use StoredSessionState::*;
        assert_eq!(evaluate_stored_session(false, None, "key"), NotLoggedIn);
        assert_eq!(evaluate_stored_session(true, None, "key"), KeyChanged);
        assert_eq!(
            evaluate_stored_session(true, Some("old"), "key"),
            KeyChanged
        );
        assert_eq!(evaluate_stored_session(true, Some("key"), "key"), Valid);
        assert_eq!(evaluate_stored_session(true, Some("key"), ""), KeyChanged);
    }
    #[test]
    fn signature_cache_reuses_then_resigns_once() {
        let sig = || Signature {
            api_key: "0123456789abcdef0123456789abcdef".into(),
            api_sig: "0000000000000000000000000000abcd".into(),
        };
        let mut c = TokenSignatureCache::new();
        let mut fetches = 0;
        for _ in 0..100 {
            if c.get().is_none() {
                fetches += 1;
                c.store(sig());
            }
            assert_eq!(c.on_error(&lf(14)), PollAction::Retry);
        }
        assert_eq!(fetches, 1);
        let mut c = TokenSignatureCache::new();
        fetches = 0;
        let mut actions = vec![];
        for e in [lf(13), lf(14)] {
            if c.get().is_none() {
                fetches += 1;
                c.store(sig());
            }
            actions.push(c.on_error(&e));
        }
        assert_eq!(fetches, 2);
        assert_eq!(
            actions
                .iter()
                .filter(|a| **a == PollAction::RetryWithFreshSignature)
                .count(),
            1
        );
        let mut c = TokenSignatureCache::new();
        c.store(sig());
        assert!(matches!(
            c.on_error(&lf(13)),
            PollAction::RetryWithFreshSignature
        ));
        assert!(matches!(c.on_error(&lf(13)), PollAction::Stop(_)));
        assert_eq!(
            c.on_error(&AuthError::Sign(SignError::Transport("offline".into()))),
            PollAction::Retry
        );
        let unavailable = AuthError::Sign(SignError::Unavailable {
            status: 403,
            code: "app_disabled".into(),
        });
        match c.on_error(&unavailable) {
            PollAction::Stop(m) => assert!(m.contains("署名サービスが使えません")),
            _ => panic!("expected stop"),
        }
        for e in [lf(10), lf(26)] {
            match c.on_error(&e) {
                PollAction::Stop(m) => assert!(m.contains("サービス側の問題")),
                _ => panic!("expected stop"),
            }
        }
    }
    #[test]
    fn every_sign_error_has_a_user_message() {
        let cases = [
            SignError::Unavailable {
                status: 403,
                code: "x".into(),
            },
            SignError::Transport("x".into()),
            SignError::RateLimited {
                retry_after_secs: Some(3),
            },
            SignError::ServerError {
                status: 503,
                code: None,
            },
            SignError::Rejected {
                status: 400,
                code: None,
            },
            SignError::InvalidInput("format".into()),
            SignError::BadResponse("invalid".into()),
        ];
        assert!(cases.iter().all(|e| !e.user_message().is_empty()));
    }
}
