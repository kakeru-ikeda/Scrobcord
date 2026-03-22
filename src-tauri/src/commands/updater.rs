use serde::Serialize;
use tauri_plugin_opener::OpenerExt;

/// GitHub Releases で管理するリポジトリのオーナー/名前
const GITHUB_REPO: &str = "kakeru-ikeda/Scrobcord";

/// GitHub Releases API のエンドポイント
const GITHUB_API_LATEST: &str =
    "https://api.github.com/repos/kakeru-ikeda/Scrobcord/releases/latest";

#[derive(Serialize)]
pub struct UpdateInfo {
    pub available: bool,
    pub latest_version: String,
    pub current_version: String,
    pub release_url: String,
}

/// GitHub Releases API でアップデートを確認する
///
/// ネットワークエラーやリリース未公開（404）の場合は `available: false` を返し、
/// フロントエンドにエラーを伝播させない。
#[tauri::command]
pub async fn check_for_updates() -> Result<UpdateInfo, String> {
    let current = env!("CARGO_PKG_VERSION").to_string();

    let client = reqwest::Client::builder()
        .user_agent(format!("Scrobcord/{} ({})", current, GITHUB_REPO))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(GITHUB_API_LATEST)
        .send()
        .await
        .map_err(|e| format!("ネットワークエラー: {}", e))?;

    // リリースがまだ一件もない場合 (404)
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(UpdateInfo {
            available: false,
            latest_version: current.clone(),
            current_version: current,
            release_url: String::new(),
        });
    }

    // その他の HTTP エラー
    if !response.status().is_success() {
        return Err(format!(
            "GitHub API エラー: HTTP {}",
            response.status().as_u16()
        ));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("レスポンスのパースに失敗: {}", e))?;

    let tag_name = json["tag_name"].as_str().unwrap_or("").to_string();
    let latest_version = tag_name.trim_start_matches('v').to_string();
    let release_url = json["html_url"].as_str().unwrap_or("").to_string();

    let available = is_newer_version(&latest_version, &current);

    log::info!(
        "アップデート確認: current={} latest={} available={}",
        current,
        latest_version,
        available
    );

    Ok(UpdateInfo {
        available,
        latest_version,
        current_version: current,
        release_url,
    })
}

/// GitHub Releases ページをデフォルトブラウザで開く
///
/// URL が `https://github.com/` 始まりであることを確認してから開く（SSRF 対策）。
#[tauri::command]
pub fn open_release_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    if !url.starts_with("https://github.com/") {
        return Err("不正な URL です".to_string());
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

/// セマンティックバージョン比較: `latest` が `current` より新しいか
///
/// プレリリースサフィックス（例: "0.2.0-beta"）は数値部分のみで比較する。
fn is_newer_version(latest: &str, current: &str) -> bool {
    fn parse_version(v: &str) -> (u32, u32, u32) {
        // "-beta" などのプレリリース部分を除去
        let numeric = v.split('-').next().unwrap_or(v);
        let parts: Vec<u32> = numeric.split('.').map(|s| s.parse().unwrap_or(0)).collect();
        (
            parts.first().copied().unwrap_or(0),
            parts.get(1).copied().unwrap_or(0),
            parts.get(2).copied().unwrap_or(0),
        )
    }
    parse_version(latest) > parse_version(current)
}

#[cfg(test)]
mod tests {
    use super::is_newer_version;

    #[test]
    fn newer_patch_is_detected() {
        assert!(is_newer_version("0.1.1", "0.1.0"));
    }

    #[test]
    fn newer_minor_is_detected() {
        assert!(is_newer_version("0.2.0", "0.1.9"));
    }

    #[test]
    fn same_version_is_not_newer() {
        assert!(!is_newer_version("0.1.0", "0.1.0"));
    }

    #[test]
    fn older_version_is_not_newer() {
        assert!(!is_newer_version("0.0.9", "0.1.0"));
    }

    #[test]
    fn prerelease_suffix_ignored_in_comparison() {
        assert!(!is_newer_version("0.1.0-beta", "0.1.0"));
        assert!(is_newer_version("0.2.0-beta", "0.1.0"));
    }
}
