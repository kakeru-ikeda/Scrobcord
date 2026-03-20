fn main() {
    // .env ファイルが存在すればビルド時環境変数として読み込む
    // → option_env!("LASTFM_API_KEY") 等で参照可能になる
    println!("cargo:rerun-if-changed=.env");
    if let Ok(contents) = std::fs::read_to_string(".env") {
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');
                println!("cargo:rustc-env={key}={value}");
            }
        }
    }

    tauri_build::build()
}
