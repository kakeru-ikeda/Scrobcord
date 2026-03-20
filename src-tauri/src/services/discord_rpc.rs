use std::io::{Read, Write};

use serde_json::{json, Value};

use crate::models::{settings::Settings, track::Track};

// ---------------------------------------------------------------------------
// Discord IPC フレーム Op コード
// ---------------------------------------------------------------------------
const OP_HANDSHAKE: u32 = 0;
const OP_FRAME: u32 = 1;
const OP_CLOSE: u32 = 2;
const OP_PING: u32 = 3;
const OP_PONG: u32 = 4;

// ---------------------------------------------------------------------------
// ReadWrite トレイト（Box<dyn ReadWrite> を Send にするため Send をスーパートレイト）
// ---------------------------------------------------------------------------
pub trait ReadWrite: Read + Write + Send {}
impl<T: Read + Write + Send> ReadWrite for T {}

// ---------------------------------------------------------------------------
// フレーム I/O
// ヘッダー: [op: u32 LE, len: u32 LE] (8 bytes) + JSON ペイロード
// ---------------------------------------------------------------------------
fn write_frame(w: &mut dyn Write, op: u32, payload: &Value) -> Result<(), String> {
    let data = serde_json::to_vec(payload).map_err(|e| e.to_string())?;
    let len = data.len() as u32;
    w.write_all(&op.to_le_bytes()).map_err(|e| e.to_string())?;
    w.write_all(&len.to_le_bytes()).map_err(|e| e.to_string())?;
    w.write_all(&data).map_err(|e| e.to_string())?;
    w.flush().map_err(|e| e.to_string())
}

fn read_frame(r: &mut dyn Read) -> Result<(u32, Value), String> {
    let mut header = [0u8; 8];
    r.read_exact(&mut header).map_err(|e| e.to_string())?;
    let op = u32::from_le_bytes(header[0..4].try_into().unwrap());
    let len = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
    // サニティチェック — 64KB 超はプロトコル違反
    if len > 65_536 {
        return Err("Discord IPC frame too large".to_string());
    }
    let mut data = vec![0u8; len];
    r.read_exact(&mut data).map_err(|e| e.to_string())?;
    serde_json::from_slice(&data)
        .map(|v| (op, v))
        .map_err(|e| e.to_string())
}

fn read_until_ready(stream: &mut dyn ReadWrite) -> Result<(), String> {
    for _ in 0..8 {
        let (op, resp) = read_frame(stream)?;
        match op {
            OP_CLOSE => return Err(format!("Discord が接続を閉じました: {resp}")),
            OP_PING => {
                write_frame(stream, OP_PONG, &resp)?;
            }
            OP_FRAME => {
                if resp["cmd"].as_str() == Some("DISPATCH") && resp["evt"].as_str() == Some("READY")
                {
                    return Ok(());
                }
                if resp["evt"].as_str() == Some("ERROR") {
                    return Err(format!("Discord READY error: {resp}"));
                }
            }
            _ => {}
        }
    }

    Err("Discord READY 応答を受信できませんでした".to_string())
}

fn send_rpc_command(stream: &mut dyn ReadWrite, payload: &Value) -> Result<(), String> {
    log::debug!("Sending RPC command: {}", payload);
    write_frame(stream, OP_FRAME, payload)?;

    for _ in 0..8 {
        let (op, resp) = read_frame(stream)?;
        log::debug!("Received RPC frame: op={}, data={}", op, resp);
        match op {
            OP_CLOSE => return Err(format!("Discord が接続を閉じました: {resp}")),
            OP_PING => {
                write_frame(stream, OP_PONG, &resp)?;
            }
            OP_FRAME => {
                if resp["evt"].as_str() == Some("ERROR") {
                    // DiscordのRPCエラー（文字数制限など）はこれで検知できるはず
                    return Err(format!("Discord RPC error: {resp}"));
                }
                if resp["nonce"] == payload["nonce"] {
                    return Ok(());
                }
            }
            _ => {}
        }
    }

    Err("Discord RPC 応答を受信できませんでした".to_string())
}

// ---------------------------------------------------------------------------
// プラットフォーム別接続関数
// ---------------------------------------------------------------------------
#[cfg(target_os = "windows")]
fn open_connection() -> Result<Box<dyn ReadWrite>, String> {
    use std::fs::OpenOptions;
    for i in 0..10u32 {
        let path = format!(r"\\.\pipe\discord-ipc-{}", i);
        if let Ok(f) = OpenOptions::new().read(true).write(true).open(&path) {
            return Ok(Box::new(f));
        }
    }
    Err("Discordが起動していません（IPC pipe が見つかりません）".to_string())
}

#[cfg(not(target_os = "windows"))]
fn open_connection() -> Result<Box<dyn ReadWrite>, String> {
    let dirs: Vec<String> = {
        let mut d = Vec::new();
        if let Ok(v) = std::env::var("XDG_RUNTIME_DIR") {
            d.push(v);
        }
        if let Ok(v) = std::env::var("TMPDIR") {
            d.push(v);
        }
        d.push("/tmp".to_string());
        d
    };
    for dir in &dirs {
        for i in 0..10u32 {
            let path = format!("{}/discord-ipc-{}", dir, i);
            if let Ok(s) = std::os::unix::net::UnixStream::connect(&path) {
                return Ok(Box::new(s));
            }
        }
    }
    Err("Discordが起動していません（IPC socket が見つかりません）".to_string())
}

// ---------------------------------------------------------------------------
// フォーマット文字列のホワイトリスト置換（任意コード実行防止）
// ---------------------------------------------------------------------------
fn format_rpc(template: &str, track: &Track) -> String {
    template
        .replace("{track}", &track.title)
        .replace("{artist}", &track.artist)
        .replace("{album}", &track.album)
}

// ---------------------------------------------------------------------------
// 文字列バリデーション（Discord RPC は 2〜128 文字を要求する）
// URL用の場合は長さを最大512まで許容し、...で丸めないように別関数にする
// ---------------------------------------------------------------------------
fn valid_rpc_str(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let chars: Vec<char> = s.chars().collect();
    if chars.len() < 2 {
        return Some(format!("{} ", s)); // pad
    }
    if chars.len() > 128 {
        let mut t: String = chars.into_iter().take(127).collect();
        t.push('…');
        return Some(t);
    }
    Some(s.to_string())
}

fn valid_rpc_url(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let chars: Vec<char> = s.chars().collect();
    // Discord RPC の URL 上限は通常512文字。
    // 日本語URLエンコードが含まれると非常に長くなるが、途中カットすると無効なURIになる。
    if chars.len() > 512 {
        return None; // カットせずに送信を諦めるか、後述の対応を行う
    }
    Some(s.to_string())
}

// ---------------------------------------------------------------------------
// ビルトイン Application ID（.env の DISCORD_APP_ID から取得）
// ユーザーが設定しなくても動くようにするためのデフォルト値
// ---------------------------------------------------------------------------
const BUILTIN_APP_ID: &str = match option_env!("DISCORD_APP_ID") {
    Some(id) => id,
    None => "",
};

// ---------------------------------------------------------------------------
// DiscordRpcClient
// ---------------------------------------------------------------------------
pub struct DiscordRpcClient {
    pub app_id: String,
    stream: Option<Box<dyn ReadWrite>>,
    nonce: u64,
}

impl DiscordRpcClient {
    pub fn new(app_id: String) -> Self {
        Self {
            app_id,
            stream: None,
            nonce: 0,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    /// Discord IPC に接続して handshake を完了する
    pub fn connect(&mut self) -> Result<(), String> {
        // ユーザー設定の app_id が空ならビルトイン ID を使用
        let effective_id = if self.app_id.is_empty() {
            if BUILTIN_APP_ID.is_empty() {
                return Err(
                    "Discord Application ID が設定されていません（ビルトイン ID もありません）"
                        .to_string(),
                );
            }
            BUILTIN_APP_ID.to_string()
        } else {
            self.app_id.clone()
        };

        // 既存接続があれば切断
        self.disconnect();

        // 実際に使用する app_id を保持（disconnect 時の close frame にも使う）
        self.app_id = effective_id.clone();

        let mut stream = open_connection()?;

        // Handshake (op=0)
        write_frame(
            &mut stream,
            OP_HANDSHAKE,
            &json!({ "v": 1, "client_id": effective_id }),
        )?;

        // READY レスポンス待ち（PING/PONG も処理）
        read_until_ready(&mut stream)?;

        self.stream = Some(stream);
        Ok(())
    }

    /// Discord Rich Presence にトラック情報をセットする
    pub fn set_activity(&mut self, track: &Track, settings: &Settings) -> Result<(), String> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| "Discordに接続されていません".to_string())?;

        let mut activity = serde_json::Map::new();

        if let Some(details) = valid_rpc_str(&format_rpc(&settings.rpc_details_format, track)) {
            activity.insert("details".to_string(), details.into());
        }

        if let Some(state_str) = valid_rpc_str(&format_rpc(&settings.rpc_state_format, track)) {
            activity.insert("state".to_string(), state_str.into());
        }

        if settings.rpc_show_album_art {
            if let Some(ref url) = track.album_art_url {
                if !url.is_empty() {
                    let mut assets = serde_json::Map::new();
                    assets.insert("large_image".to_string(), url.clone().into());

                    if let Some(large_text) = valid_rpc_str(&track.album) {
                        assets.insert("large_text".to_string(), large_text.into());
                    }

                    assets.insert("small_image".to_string(), "lastfm".into());
                    assets.insert("small_text".to_string(), "Scrobcord".into());

                    activity.insert("assets".to_string(), assets.into());
                }
            }
        }

        if settings.rpc_show_timestamp {
            let mut timestamps = serde_json::Map::new();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            timestamps.insert("start".to_string(), now.into());
            activity.insert("timestamps".to_string(), timestamps.into());
        }

        if settings.rpc_show_lastfm_button {
            if let Some(ref url) = track.url {
                if !url.is_empty() {
                    // Last.fmへのリンク(URLエンコードにより長くなりがち)
                    if let Some(url_str) = valid_rpc_url(url) {
                        activity.insert(
                            "buttons".to_string(),
                            json!([{
                                "label": "Last.fmで開く",
                                "url":   url_str
                            }]),
                        );
                    } else if url.len() > 512 {
                        // URLが長すぎる場合はアーティストだけのURLにフォールバックするなどの工夫ができるが、
                        // まずはボタンを省略することでエラーを回避する
                        // activity.insert("buttons".to_string(), json!([{
                        //    "label": "Last.fmで開く",
                        //    "url":   "https://www.last.fm/" // 究極のフォールバック
                        // }]));
                    }
                }
            }
        }

        self.nonce += 1;
        let payload = json!({
            "cmd": "SET_ACTIVITY",
            "args": {
                "pid": std::process::id(),
                "activity": activity
            },
            "nonce": self.nonce.to_string()
        });

        send_rpc_command(stream, &payload)
    }

    /// Rich Presence をクリアする（再生停止時）
    pub fn clear_activity(&mut self) -> Result<(), String> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| "Discordに接続されていません".to_string())?;

        self.nonce += 1;
        let payload = json!({
            "cmd": "SET_ACTIVITY",
            "args": {
                "pid": std::process::id()
            },
            "nonce": self.nonce.to_string()
        });

        send_rpc_command(stream, &payload)
    }

    /// ダミーのPINGフレームを送信して接続の生死を確認する
    pub fn ping(&mut self) -> Result<(), String> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| "Discordに接続されていません".to_string())?;

        // nonceを付与してPING送信
        self.nonce += 1;
        let payload = json!({
            "nonce": self.nonce.to_string()
        });

        // PING なので OP=3
        write_frame(stream, OP_PING, &payload)?;

        // PING応答は非同期に来る可能性があるためここでは受信待ちせず、
        // 書込(write_frame)結果だけでOSレベルのパイプ切断を検知できればヨシとする。
        // パイプが壊れていれば write_frame がエラーになって Err("Broken pipe") 等が返る
        Ok(())
    }

    /// 接続を切断する
    pub fn disconnect(&mut self) {
        if let Some(mut stream) = self.stream.take() {
            let _ = write_frame(
                &mut stream,
                OP_CLOSE,
                &json!({ "v": 1, "client_id": self.app_id }),
            );
        }
    }
}
