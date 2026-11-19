use std::collections::BTreeMap;
use std::time::Duration;

pub const SCROBSIGN_URL: &str = "https://scrobsign.sukisuki.club";
pub const RESERVED_PARAMS: [&str; 5] = ["api_key", "api_sig", "method", "format", "callback"];

pub fn sign_endpoint() -> String {
    format!("{SCROBSIGN_URL}/v1/scrobcord/sign")
}

pub async fn request_signature(
    client: &reqwest::Client,
    method: &str,
    params: &BTreeMap<String, String>,
) -> Result<Signature, SignError> {
    let body = build_sign_request_body(method, params)?;
    let response = client
        .post(sign_endpoint())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .timeout(Duration::from_secs(15))
        .body(body)
        .send()
        .await
        .map_err(|e| SignError::Transport(e.without_url().to_string()))?;
    let status = response.status().as_u16();
    let retry_after = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let body = response
        .text()
        .await
        .map_err(|e| SignError::Transport(e.without_url().to_string()))?;
    parse_sign_response(status, retry_after.as_deref(), &body)
}

#[derive(Clone, PartialEq, Eq)]
pub struct Signature {
    pub api_key: String,
    pub api_sig: String,
}

impl std::fmt::Debug for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Signature")
            .field("api_key", &self.api_key)
            .field("api_sig", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignError {
    Unavailable { status: u16, code: String },
    Transport(String),
    RateLimited { retry_after_secs: Option<u64> },
    ServerError { status: u16, code: Option<String> },
    Rejected { status: u16, code: Option<String> },
    InvalidInput(String),
    BadResponse(String),
}

impl SignError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Transport(_) | Self::RateLimited { .. } | Self::ServerError { .. }
        )
    }

    pub fn user_message(&self) -> String {
        match self {
            Self::Unavailable { status, code } => format!("署名サービスが使えません（HTTP {status} {code}）。アプリを最新版に更新するか、時間をおいて再度お試しください"),
            Self::Transport(d) => format!("署名サービスに接続できません（{d}）。しばらくしてから再度お試しください"),
            Self::RateLimited { retry_after_secs: Some(n) } => format!("署名サービスが混み合っています（{n} 秒後に再試行できます）"),
            Self::RateLimited { retry_after_secs: None } => "署名サービスが混み合っています（しばらくしてから再度お試しください）".into(),
            Self::ServerError { status, code } => format!("署名サービスで一時的な障害が起きています（HTTP {status} {}）", code.as_deref().unwrap_or("-")),
            Self::Rejected { status, code } => format!("署名サービスがリクエストを拒否しました（HTTP {status} {}）", code.as_deref().unwrap_or("-")),
            Self::InvalidInput(name) => format!("署名サービスへ送れないパラメータです（{name}）"),
            Self::BadResponse(d) => format!("署名サービスの応答が不正です（{d}）"),
        }
    }
}

impl std::fmt::Display for SignError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.user_message())
    }
}

pub fn build_sign_request_body(
    method: &str,
    params: &BTreeMap<String, String>,
) -> Result<String, SignError> {
    if let Some(key) = params
        .keys()
        .find(|key| RESERVED_PARAMS.contains(&key.as_str()))
    {
        return Err(SignError::InvalidInput(key.clone()));
    }
    serde_json::to_string(&serde_json::json!({"method": method, "params": params}))
        .map_err(|_| SignError::BadResponse("request serialization failed".into()))
}

pub fn parse_sign_response(
    status: u16,
    retry_after: Option<&str>,
    body: &str,
) -> Result<Signature, SignError> {
    let parsed = serde_json::from_str::<serde_json::Value>(body).ok();
    let code = parsed
        .as_ref()
        .and_then(|v| v["error"]["code"].as_str())
        .map(str::to_owned);
    match status {
        200 => {
            let value = parsed.ok_or_else(|| SignError::BadResponse("non-JSON response".into()))?;
            let Some(api_key) = value["api_key"].as_str() else {
                return Err(SignError::BadResponse("missing api_key".into()));
            };
            let Some(api_sig) = value["api_sig"].as_str() else {
                return Err(SignError::BadResponse("missing api_sig".into()));
            };
            if !is_hex32(api_key) {
                return Err(SignError::BadResponse("invalid api_key".into()));
            }
            if !is_hex32(api_sig) {
                return Err(SignError::BadResponse("invalid api_sig".into()));
            }
            Ok(Signature {
                api_key: api_key.into(),
                api_sig: api_sig.into(),
            })
        }
        429 => Err(SignError::RateLimited {
            retry_after_secs: retry_after.and_then(|v| v.parse().ok()),
        }),
        500..=599 => Err(SignError::ServerError { status, code }),
        403 if matches!(code.as_deref(), Some("app_disabled" | "method_forbidden")) => {
            Err(SignError::Unavailable {
                status,
                code: code.unwrap(),
            })
        }
        404 if code.as_deref() == Some("unknown_app") => Err(SignError::Unavailable {
            status,
            code: code.unwrap(),
        }),
        400..=499 => Err(SignError::Rejected { status, code }),
        _ => Err(SignError::BadResponse(format!(
            "unexpected status {status}"
        ))),
    }
}

fn is_hex32(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;
    const KEY: &str = "0123456789abcdef0123456789abcdef";
    const SIG: &str = "0000000000000000000000000000abcd";
    #[test]
    fn endpoint_and_request_body() {
        assert_eq!(
            sign_endpoint(),
            "https://scrobsign.sukisuki.club/v1/scrobcord/sign"
        );
        let p = BTreeMap::new();
        let v: serde_json::Value =
            serde_json::from_str(&build_sign_request_body("auth.getToken", &p).unwrap()).unwrap();
        assert_eq!(v, serde_json::json!({"method":"auth.getToken","params":{}}));
        let p = BTreeMap::from([("token".to_string(), "dummytoken".to_string())]);
        let v: serde_json::Value =
            serde_json::from_str(&build_sign_request_body("auth.getSession", &p).unwrap()).unwrap();
        assert_eq!(
            v,
            serde_json::json!({"method":"auth.getSession","params":{"token":"dummytoken"}})
        );
    }
    #[test]
    fn rejects_reserved_parameters() {
        for k in ["api_key", "format"] {
            let p = BTreeMap::from([(k.into(), "x".into())]);
            assert!(matches!(
                build_sign_request_body("x", &p),
                Err(SignError::InvalidInput(_))
            ));
        }
    }
    #[test]
    fn parses_success_and_redacts_debug() {
        let s = parse_sign_response(
            200,
            None,
            &format!(r#"{{"api_key":"{KEY}","api_sig":"{SIG}"}}"#),
        )
        .unwrap();
        assert_eq!(s.api_key, KEY);
        assert!(!format!("{s:?}").contains(SIG));
    }
    #[test]
    fn rejects_malformed_success_without_echoing_signature() {
        for b in [
            r#"{"api_key":"0123456789abcdef0123456789abcdef"}"#,
            r#"{"api_key":"0123456789abcdef0123456789abcdef","api_sig":"0000000000000000000000000000abcg"}"#,
            "not json",
        ] {
            let e = parse_sign_response(200, None, b).unwrap_err();
            assert!(matches!(e, SignError::BadResponse(_)));
            assert!(!e.to_string().contains(SIG));
        }
    }
    #[test]
    fn classifies_service_statuses() {
        assert!(matches!(
            parse_sign_response(429, Some("30"), ""),
            Err(SignError::RateLimited {
                retry_after_secs: Some(30)
            })
        ));
        assert!(matches!(
            parse_sign_response(429, None, ""),
            Err(SignError::RateLimited {
                retry_after_secs: None
            })
        ));
        let e = parse_sign_response(503, None, r#"{"error":{"code":"app_misconfigured"}}"#)
            .unwrap_err();
        assert!(matches!(e, SignError::ServerError { .. }));
        assert!(e.is_retryable());
        for (status, code) in [
            (403, "app_disabled"),
            (403, "method_forbidden"),
            (404, "unknown_app"),
        ] {
            let e =
                parse_sign_response(status, None, &format!(r#"{{"error":{{"code":"{code}"}}}}"#))
                    .unwrap_err();
            assert!(matches!(e, SignError::Unavailable { .. }));
            assert!(!e.is_retryable());
        }
        assert!(matches!(
            parse_sign_response(400, None, r#"{"error":{"code":"invalid_params"}}"#),
            Err(SignError::Rejected { .. })
        ));
    }
}
