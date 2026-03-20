use std::io::{Read, Write};

use serde_json::{json, Value};

use crate::models::{settings::Settings, track::Track};

// ---------------------------------------------------------------------------
// Discord IPC フレーム Op コード
// ---------------------------------------------------------------------------
const OP_HANDSHAKE: u32 = 0;
const OP_FRAME: u32 = 1;
const OP_CLOSE: u32 = 2;

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

        let mut stream = open_connection()?;

        // Handshake (op=0)
        write_frame(
            &mut stream,
            OP_HANDSHAKE,
            &json!({ "v": 1, "client_id": effective_id }),
        )?;

        // READY レスポンス待ち
        let (op, resp) = read_frame(&mut stream)?;
        if op == OP_CLOSE {
            return Err(format!("Discord が接続を閉じました: {resp}"));
        }
        if resp["cmd"].as_str() != Some("DISPATCH") || resp["evt"].as_str() != Some("READY") {
            return Err(format!("READY を期待しましたが受信: {resp}"));
        }

        self.stream = Some(stream);
        Ok(())
    }

    /// Discord Rich Presence にトラック情報をセットする
    pub fn set_activity(&mut self, track: &Track, settings: &Settings) -> Result<(), String> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| "Discordに接続されていません".to_string())?;

        let mut activity = json!({
            "details": format_rpc(&settings.rpc_details_format, track),
            "state":   format_rpc(&settings.rpc_state_format, track),
        });

        if settings.rpc_show_album_art {
            if let Some(ref url) = track.album_art_url {
                if !url.is_empty() {
                    activity["assets"] = json!({
                        "large_image": url,
                        "large_text":  track.album,
                        "small_image": "lastfm",
                    });
                }
            }
        }

        if settings.rpc_show_timestamp {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            activity["timestamps"] = json!({ "start": now });
        }

        if settings.rpc_show_lastfm_button {
            if let Some(ref url) = track.url {
                if !url.is_empty() {
                    activity["buttons"] = json!([{
                        "label": "Last.fmで開く",
                        "url":   url
                    }]);
                }
            }
        }

        self.nonce += 1;
        write_frame(
            stream,
            OP_FRAME,
            &json!({
                "cmd": "SET_ACTIVITY",
                "args": {
                    "pid": std::process::id(),
                    "activity": activity
                },
                "nonce": self.nonce.to_string()
            }),
        )
    }

    /// Rich Presence をクリアする（再生停止時）
    pub fn clear_activity(&mut self) -> Result<(), String> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| "Discordに接続されていません".to_string())?;

        self.nonce += 1;
        write_frame(
            stream,
            OP_FRAME,
            &json!({
                "cmd": "SET_ACTIVITY",
                "args": {
                    "pid": std::process::id(),
                    "activity": null
                },
                "nonce": self.nonce.to_string()
            }),
        )
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
