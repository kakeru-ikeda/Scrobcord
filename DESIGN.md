# Scrobcord 設計ドキュメント

> Last.fm scrobble → Discord Rich Presence 送信に特化したクロスプラットフォーム デスクトップアプリ

---

## 概要

| 項目            | 内容                                                                   |
| --------------- | ---------------------------------------------------------------------- |
| **アプリ名**    | Scrobcord                                                              |
| **目的**        | Last.fm APIで取得した現在再生中の楽曲をDiscord Rich Presenceへ送信する |
| **対象OS**      | Windows / macOS / Linux                                                |
| **Discord Bot** | 不要（Rich Presence は IPC ローカル通信のみ）                          |

---

## 技術スタック

| レイヤー             | 技術                             |
| -------------------- | -------------------------------- |
| デスクトップフレーム | Tauri 2.x                        |
| バックエンド         | Rust (tokio 非同期)              |
| フロントエンド       | React 18 + TypeScript + Vite     |
| UI                   | Tailwind CSS + shadcn/ui         |
| 状態管理             | Zustand                          |
| 設定永続化           | tauri-plugin-store (JSON)        |
| セキュア資格情報     | keyring crate（OS キーチェーン） |

---

## プロジェクト構成

```
scrobcord/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs                  # エントリポイント・Tauri builder
│       ├── lib.rs                   # コマンド登録
│       ├── commands/
│       │   ├── mod.rs
│       │   ├── auth.rs              # Last.fm 認証フロー
│       │   ├── discord.rs           # Discord RPC 制御
│       │   ├── polling.rs           # ポーリング制御
│       │   └── settings.rs          # 設定 CRUD
│       ├── services/
│       │   ├── mod.rs
│       │   ├── lastfm.rs            # Last.fm API クライアント
│       │   ├── discord_rpc.rs       # Discord RPC IPC クライアント
│       │   └── poller.rs            # ポーリングループ (tokio task)
│       ├── models/
│       │   ├── track.rs             # 楽曲情報
│       │   ├── settings.rs          # 設定構造体
│       │   └── status.rs            # 接続ステータス
│       └── state.rs                 # AppState (Arc<Mutex<...>>)
│
├── src/
│   ├── main.tsx
│   ├── App.tsx
│   ├── pages/
│   │   ├── Dashboard.tsx            # ナウプレイング表示・接続状態
│   │   └── Settings.tsx             # タブ付き設定画面
│   ├── components/
│   │   ├── NowPlayingCard.tsx
│   │   ├── ConnectionStatus.tsx
│   │   └── settings/
│   │       ├── LastfmSettings.tsx
│   │       ├── DiscordSettings.tsx
│   │       └── GeneralSettings.tsx
│   ├── hooks/
│   │   ├── useNowPlaying.ts
│   │   └── useConnectionStatus.ts
│   ├── store/
│   │   └── appStore.ts
│   └── lib/
│       └── tauriInvoke.ts
│
├── .github/
│   └── workflows/
│       └── build.yml
├── package.json
├── vite.config.ts
└── tsconfig.json
```

---

## Rust Crates

```toml
[dependencies]
tauri                = { version = "2", features = ["tray-icon", "image-png"] }
tauri-plugin-store   = "2"
tauri-plugin-shell   = "2"
tauri-plugin-autostart = "2"
tokio                = { version = "1", features = ["full"] }
reqwest              = { version = "0.12", features = ["json"] }
serde                = { version = "1", features = ["derive"] }
serde_json           = "1"
md5                  = "0.7"    # Last.fm API 署名
keyring              = "2"      # OS キーチェーン（session_key / api_secret 保存）
chrono               = { version = "0.4", features = ["serde"] }
log                  = "0.4"
tauri-plugin-log     = "2"
```

> `discord-rich-presence` crate を優先使用。メンテ状況次第では Discord IPC プロトコル（JSON over Unix Socket / Named Pipe）を直接実装。

---

## Last.fm 認証フロー

> API Key / API Secret はビルド時に環境変数 `LASTFM_API_KEY` / `LASTFM_API_SECRET` でバイナリに埋め込む。
> ユーザーは API キーを入力不要。

```
[「Last.fm でログイン」クリック]
        │
        ▼
[Rust] GET auth.getToken (API Key + MD5 署名)
        │  → 一時トークン取得
        ▼
[Rust] shell::open でブラウザ起動
       https://www.last.fm/api/auth/?api_key=XXX&token=TOKEN
        │
        ▼
[Rust] tokio::spawn で認証ポーリング開始（3秒間隔 / 最大5分）
[イベント] lastfm-auth-polling { polling: true } → UI にスピナー表示
        │
        ▼ (ループ)
[Rust] GET auth.getSession → エラー14(未承認)なら継続 / 成功なら↓
        │
        ▼
[ユーザーがブラウザで承認すると次のポーリングで自動検出]
        │
        ▼
[Rust] session_key を OS キーチェーンに保存 (keyring)
       username を tauri-plugin-store へ永続化
        │
        ▼
[イベント] lastfm-status-changed { authenticated: true, username } → UI 更新
[イベント] lastfm-auth-polling { polling: false } → スピナー非表示
```

- ユーザーはブラウザで承認するだけでよく、アプリへの手動操作は不要
- タイムアウト時またはキャンセル時は `lastfm-auth-polling { polling: false }` を emit
- `lastfm_cancel_auth` コマンドでポーリングをキャンセル可能

---

## Discord RPC フロー

```
[アプリ起動 or 「接続」ボタン]
        │
        ▼
[Rust] IPC ソケット接続
       Windows : \\.\pipe\discord-ipc-{0..9}
       Mac/Linux: $TMPDIR/discord-ipc-{0..9}
        │
        ▼
[Rust] handshake { v:1, client_id: APP_ID }
        │
        ▼
[ポーリングループ tokio::spawn]
  Last.fm → 曲取得 → 前回と比較 → 変化あり?
        │ Yes
        ▼
[Rust] SET_ACTIVITY 送信
  {
    details   : "{artist} - {track}",
    state     : "{album}",
    assets: {
      large_image: "{album_art_url}",
      large_text : "{album}",
      small_image: "lastfm_icon",
    },
    timestamps: { start: scrobble_timestamp },
    buttons: [{ label: "Last.fm で開く", url: track_url }]
  }
        │
        ▼
[イベント] track-changed → React UI 更新
```

---

## Tauri コマンド一覧

```rust
// 認証
lastfm_get_auth_token() -> Result<String, String>
lastfm_get_session(token: String) -> Result<(), String>
lastfm_logout() -> Result<(), String>
lastfm_get_auth_status() -> AuthStatus

// ポーリング
start_polling(app: AppHandle) -> Result<(), String>
stop_polling() -> Result<(), String>

// Discord RPC
discord_connect() -> Result<(), String>
discord_disconnect() -> Result<(), String>
discord_get_status() -> DiscordStatus

// 設定
get_settings() -> Settings
save_settings(settings: Settings) -> Result<(), String>

// ナウプレイング
get_now_playing() -> Option<Track>
```

## Tauri イベント（Rust → React）

| イベント名               | ペイロード                                   |
| ------------------------ | -------------------------------------------- |
| `track-changed`          | `{ track: Track \| null }`                   |
| `discord-status-changed` | `{ connected: bool, error?: string }`        |
| `lastfm-status-changed`  | `{ authenticated: bool, username?: string }` |
| `polling-status-changed` | `{ running: bool }`                          |
| `lastfm-auth-polling`    | `{ polling: bool }`                          |

---

## 設定項目（Settings 構造体）

```rust
pub struct Settings {
    // Last.fm
    pub lastfm_username: String,     // 認証後に自動設定

    // Discord RPC
    pub discord_app_id: String,
    pub rpc_enabled: bool,           // false にすると Discord への送信をスキップ（多PC運用時に便利）
    pub rpc_details_format: String,  // 例: "{artist} - {track}"
    pub rpc_state_format: String,    // 例: "{album}"
    pub rpc_show_album_art: bool,
    pub rpc_show_timestamp: bool,
    pub rpc_show_lastfm_button: bool,

    // General
    pub poll_interval_secs: u64,     // デフォルト 15（5〜60）
    pub start_on_login: bool,
    pub minimize_to_tray: bool,
    pub language: String,            // "ja" | "en"
}
```

**フォーマットプレースホルダー:** `{track}` `{artist}` `{album}`

---

## UI 画面設計

### Dashboard

```
┌────────────────────────────────────────────┐
│  Scrobcord                   [_][□][×]     │
├────────────────────────────────────────────┤
│  ┌──────┐  Pretender                       │
│  │      │  Official髭男dism               │
│  │  ART │  Album: Editorial                │
│  └──────┘                                  │
│  ● Last.fm    接続済 (kakeru-ikeda)        │
│  ● Discord    接続済 (15秒前に更新)        │
│         [停止]          [設定]             │
└────────────────────────────────────────────┘
```

### Settings タブ

**「Last.fm」** — `[Last.fm でログイン]`→ブラウザ起動、`[承認完了]`→セッション取得、ユーザー名表示、ログアウト

**「Discord RPC」** — Application ID、Details フォーマット、State フォーマット、アルバムアート toggle、タイムスタンプ toggle、Last.fm ボタン toggle、フォーマットプレビュー

**「一般」** — ポーリング間隔スライダー（5〜60s）、ログイン時起動 toggle、トレイ最小化 toggle、言語選択

---

## リリース設計

### バージョニング

| ファイル          | 形式                 | 例            |
| ----------------- | -------------------- | ------------- |
| `package.json`    | フルバージョン       | `0.1.0-beta`  |
| `tauri.conf.json` | 数値のみ（MSI 制約） | `0.1.0`       |
| `Cargo.toml`      | 数値のみ             | `0.1.0`       |
| git tag           | `v` プレフィックス   | `v0.1.0-beta` |

### リリースフロー

```
npm run release v0.1.0-beta
  → git tag v0.1.0-beta && git push origin v0.1.0-beta
  → GitHub Actions トリガー (push tags v*)
  → ubuntu / windows / macos 3並列ビルド
  → バージョン同期 (Node.js スクリプト)
  → tauri-apps/tauri-action@v0
  → GitHub Releases (Draft) に全成果物アップロード
  → 手動 Publish
```

### GitHub Actions（`.github/workflows/build.yml`）

```yaml
name: Build and Release

on:
  push:
    tags: ["v*"]
  pull_request:
  workflow_dispatch:

jobs:
  build:
    permissions:
      contents: write
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: ubuntu-22.04
          - platform: windows-latest
          - platform: macos-latest
    runs-on: ${{ matrix.platform }}

    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: "npm"
      - name: Install Rust (stable)
        uses: dtolnay/rust-toolchain@stable
      - uses: swatinem/rust-cache@v2

      - name: Install Linux dependencies
        if: matrix.platform == 'ubuntu-22.04'
        run: |
          sudo apt update
          sudo apt install -y \
            libwebkit2gtk-4.1-dev \
            libappindicator3-dev \
            librsvg2-dev \
            patchelf

      - name: Create self-signed certificate (Windows)
        if: matrix.platform == 'windows-latest'
        shell: pwsh
        run: |
          $cert = New-SelfSignedCertificate `
            -Subject "CN=Scrobcord" `
            -CertStoreLocation "Cert:\CurrentUser\My" `
            -KeyExportPolicy Exportable `
            -KeySpec Signature `
            -KeyLength 2048 -KeyAlgorithm RSA `
            -HashAlgorithm SHA256 -Type CodeSigning
          $config = "{`"bundle`":{`"windows`":{`"certificateThumbprint`":`"$($cert.Thumbprint)`"}}}"
          $utf8NoBom = New-Object System.Text.UTF8Encoding $false
          [System.IO.File]::WriteAllText("$PWD\win-sign-override.json", $config, $utf8NoBom)
          echo "TAURI_ARGS=--config win-sign-override.json" >> $env:GITHUB_ENV

      - name: Update version from tag
        if: startsWith(github.ref, 'refs/tags/')
        shell: node {0}
        env:
          TAG_NAME: ${{ github.ref_name }}
        run: |
          const fs = require('fs');
          const version = process.env.TAG_NAME.replace(/^v/, '');
          const bundleVersion = version.replace(/-.*$/, '');
          const pkg = JSON.parse(fs.readFileSync('package.json', 'utf-8'));
          pkg.version = version;
          fs.writeFileSync('package.json', JSON.stringify(pkg, null, 2) + '\n');
          const conf = JSON.parse(fs.readFileSync('src-tauri/tauri.conf.json', 'utf-8'));
          conf.version = bundleVersion;
          fs.writeFileSync('src-tauri/tauri.conf.json', JSON.stringify(conf, null, 2) + '\n');
          let cargo = fs.readFileSync('src-tauri/Cargo.toml', 'utf-8');
          cargo = cargo.replace(/^version = "[^"]*"/m, `version = "${bundleVersion}"`);
          fs.writeFileSync('src-tauri/Cargo.toml', cargo);

      - name: Install frontend dependencies
        run: npm install

      - name: Build and upload release
        if: startsWith(github.ref, 'refs/tags/')
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: Scrobcord ${{ github.ref_name }}
          releaseBody: See the assets below to download and install this version.
          releaseDraft: true
          args: ${{ env.TAURI_ARGS || '' }}

      - name: Build only
        if: ${{ !startsWith(github.ref, 'refs/tags/') }}
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          args: ${{ env.TAURI_ARGS || '' }}
```

### 配布成果物

| OS      | 形式                          |
| ------- | ----------------------------- |
| Windows | `.exe` (NSIS) + `.msi`        |
| macOS   | `.dmg` + `.app.tar.gz`        |
| Linux   | `.AppImage` + `.deb` + `.rpm` |

---

## 実装フェーズ

| Phase | 内容                                     |
| ----- | ---------------------------------------- |
| 1     | Tauri プロジェクト初期化・基本構成       |
| 2     | Last.fm API クライアント + 認証フロー    |
| 3     | Discord RPC IPC クライアント             |
| 4     | ポーリングループ (tokio task)            |
| 5     | 設定保存・読み込み (tauri-plugin-store)  |
| 6     | React ダッシュボード UI                  |
| 7     | React 設定 UI + フォーマットプレビュー   |
| 8     | トレイアイコン・スタートアップ           |
| 9     | GitHub Actions CI/CD + 3 OS リリース確認 |

---

## 補足・注意事項

1. **Discord Application ID** — Bot なしのアプリとして Developer Portal で作成するだけ。Bot トークン不要。
2. **Last.fm API Key / Secret** — ユーザー自身が取得する必要あり。UI に取得手順へのリンクを設置。
3. **アルバムアート** — `track.getInfo` の `image` フィールドから取得し `large_image` に URL 直指定。
4. **Spotify 競合** — Spotify の Discord 連携が有効だと上書きされる。将来的にスキップ機能を追加可能。
5. **セキュアストレージ** — `session_key` / `api_secret` は keyring（Windows: Credential Manager / macOS: Keychain / Linux: Secret Service）に保存。平文ファイルへの書き込み禁止。

---

## Phase 10 — Scrobble 履歴表示

### 概要

Dashboard トップページに `user.getRecentTracks` API を使ったページネーション付き再生履歴を表示する。  
認証不要エンドポイント。API key のみで呼び出し可能。

---

### API 仕様（`user.getRecentTracks`）

```
GET https://ws.audioscrobbler.com/2.0/
  ?method=user.getRecentTracks
  &user={username}
  &api_key={api_key}
  &page={page}        // 1-based, デフォルト 1
  &limit={limit}      // デフォルト 50, 最大 200
  &format=json
```

**レスポンス（JSON）**

```json
{
  "recenttracks": {
    "@attr": {
      "user": "kakeru-ikeda",
      "page": "1",
      "perPage": "20",
      "totalPages": "523",
      "total": "10451"
    },
    "track": [
      {
        "@attr": { "nowplaying": "true" },
        "name": "Pretender",
        "artist": { "#text": "Official髭男dism" },
        "album": { "#text": "Editorial" },
        "image": [{ "size": "extralarge", "#text": "https://..." }],
        "url": "https://www.last.fm/music/...",
        "date": null
      },
      {
        "name": "Subtitle",
        "artist": { "#text": "Official髭男dism" },
        "album": { "#text": "Editorial" },
        "image": [{ "size": "extralarge", "#text": "https://..." }],
        "url": "https://www.last.fm/music/...",
        "date": { "uts": "1742600400", "#text": "21 Mar 2025, 12:00" }
      }
    ]
  }
}
```

- nowplaying トラックは `@attr.nowplaying == "true"` かつ `date` フィールドが存在しない
- `@attr.total` が総 scrobble 数（文字列で返る）

---

### データモデル（Rust）

**`models/track.rs` に追加**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrobbledTrack {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_art_url: Option<String>,
    pub url: Option<String>,
    pub timestamp: Option<i64>,  // UNIX timestamp（nowplaying 時は None）
    pub now_playing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentTracksPage {
    pub tracks: Vec<ScrobbledTrack>,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
    pub total_tracks: u64,
}
```

---

### サービス層（Rust）

**`services/lastfm.rs` に追加**

```rust
pub async fn get_recent_tracks(
    &self,
    username: &str,
    page: u32,
    limit: u32,
) -> Result<RecentTracksPage, String>
```

**実装要点:**

- `user.getRecentTracks` を `page` / `limit` パラメータ付きで呼び出す（署名不要）
- `recenttracks.@attr` から `page`, `perPage`, `totalPages`, `total` を `u32` / `u64` にパース
  - API は文字列で返すため `parse::<u32>()` / `parse::<u64>()` を使う
- `recenttracks.track` が配列の場合と単一オブジェクトの場合を両方ハンドル  
  （1件だけの場合 Last.fm API は配列ではなくオブジェクトを返すことがある）
- nowplaying トラック (`@attr.nowplaying == "true"`) の `timestamp` は `None`
- 各トラックに `now_playing` フラグを設定
- 空レスポンス（`total == 0`）は空の `tracks` を返す

---

### コマンド層（Rust）

**`commands/history.rs`（新規）**

```rust
#[tauri::command]
pub async fn get_recent_tracks(
    state: tauri::State<'_, AppState>,
    page: u32,
    limit: u32,
) -> Result<RecentTracksPage, String>
```

- `AppState` から `settings.lastfm_username` を取得
- username が空の場合は `Err("Last.fm ユーザー名が未設定です")` を返す
- `LastfmClient::get_recent_tracks(username, page, limit)` を呼び出して返す

**`commands/mod.rs`**

```rust
pub mod history;
```

**`lib.rs` の `invoke_handler` に追加**

```rust
commands::history::get_recent_tracks,
```

---

### フロントエンド型定義（TypeScript）

**`lib/tauriInvoke.ts` に追加**

```typescript
export interface ScrobbledTrack {
  title: string;
  artist: string;
  album: string;
  album_art_url: string | null;
  url: string | null;
  timestamp: number | null; // UNIX timestamp（秒）
  now_playing: boolean;
}

export interface RecentTracksPage {
  tracks: ScrobbledTrack[];
  page: number;
  per_page: number;
  total_pages: number;
  total_tracks: number;
}

export const getRecentTracks = (page: number, limit: number) =>
  invoke<RecentTracksPage>("get_recent_tracks", { page, limit });
```

---

### Hook（TypeScript）

**`hooks/useScrobbleHistory.ts`（新規）**

```typescript
export function useScrobbleHistory(limit = 20) {
  const [page, setPage]       = useState(1);
  const [data, setData]       = useState<RecentTracksPage | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError]     = useState<string | null>(null);
  const authenticated         = useAppStore(s => s.lastfmStatus.authenticated);

  // ページ取得関数
  const fetchPage = useCallback(async (p: number) => { ... }, [authenticated, limit]);

  // 初回・認証状態変化時にロード
  useEffect(() => {
    if (authenticated) fetchPage(1);
    else { setData(null); setPage(1); }
  }, [authenticated]);

  // track-changed イベントで page=1 の時だけ自動リフレッシュ
  // （ポーリングが新しいscrobbleを検知→履歴最新化）
  useEffect(() => {
    const unlisten = listen<{ track: Track | null }>('track-changed', () => {
      if (page === 1) fetchPage(1);
    });
    return () => { unlisten.then(f => f()); };
  }, [page, fetchPage]);

  return { data, loading, error, page, fetchPage };
}
```

---

### コンポーネント（TypeScript）

**`components/ScrobbleHistory.tsx`（新規）**

各行の表示:

- 左: アルバムアート 32×32px（なければ Music アイコン）
- 中: 曲名（`font-medium truncate`）/ アーティスト名（`text-xs text-muted-foreground truncate`）
- 右: `now_playing` → `いま再生中` バッジ（緑）/ それ以外 → 相対時間（`dayjs().fromNow()` 相当）

```
┌─ 再生履歴 ─────────── ページ 1 / 523 ────[↻]─┐
│ ▶ [art] Pretender          Official髭男dism  いま再生中 │
│   [art] Subtitle           Official髭男dism    3 分前   │
│   [art] I LOVE...          Official髭男dism    8 分前   │
│   [art] Cry Baby           Official髭男dism   15 分前   │
│   ...（計 20 行）                                       │
├────────────────────────────────────────────────────────┤
│                     [◀ 前]  1 / 523  [次 ▶]           │
└────────────────────────────────────────────────────────┘
```

**ページネーション仕様:**

- 前/次ボタン（1ページ目で「前」disabled、最終ページで「次」disabled）
- 「X / Y ページ」テキスト表示
- ローディング中はボタンを disabled
- エラー時はエラーメッセージと再試行ボタンを表示

---

### Dashboard レイアウト変更

ウィンドウサイズ: 480×640px (resizable)

```
┌─ TitleBar (32px) ──────────────────────────────┐
├────────────────────────────────────────────────┤
│  NowPlayingCard (shrink-0, ~130px)              │
│  ※ 未認証/未再生時は ~48px の最小表示          │
├────────────────────────────────────────────────┤
│  ScrobbleHistory (flex-1 min-h-0)              │
│  ├ ヘッダー行（タイトル + ページ情報 + ↻）     │
│  ├ スクロール可能なトラックリスト (overflow-y-auto) │
│  └ ページネーションバー                        │
├────────────────────────────────────────────────┤
│  ConnectionStatus (~48px, shrink-0)             │
├────────────────────────────────────────────────┤
│  Buttons (ポーリング停止 / 設定) (~56px, shrink-0) │
└────────────────────────────────────────────────┘
```

**変更点:**

- NowPlayingCard を `shrink-0` にし、ScrobbleHistory が `flex-1 min-h-0` を取る
- ScrobbleHistory 内部のトラックリスト部分を `overflow-y-auto` でスクロール可能にする
- `limit` はデフォルト 20（1行 ~32px × 20 = ~640px → flex-1 で収まる量に自動調整）

---

### 時刻フォーマット

timestamp（UNIX秒）→ 相対表示のユーティリティを `src/lib/utils.ts` に追加:

```typescript
export function formatRelativeTime(unixSec: number): string {
  const diff = Math.floor(Date.now() / 1000) - unixSec;
  if (diff < 60) return `${diff} 秒前`;
  if (diff < 3600) return `${Math.floor(diff / 60)} 分前`;
  if (diff < 86400) return `${Math.floor(diff / 3600)} 時間前`;
  return `${Math.floor(diff / 86400)} 日前`;
}
```

---

### 追加する Tauri コマンド

```rust
// 履歴
get_recent_tracks(page: u32, limit: u32) -> Result<RecentTracksPage, String>
```

---

## Phase 11 — オンラインアップデート確認

### 概要

アプリ起動時に GitHub Releases API を呼び出して最新バージョンを確認し、新バージョンがあれば Dashboard に通知バナーを表示する。  
**自動インストールは行わない**（署名インフラ不要）。ユーザーがバナーのボタンをクリックするとリリースページをブラウザで開く。

---

### フロー

```
[アプリ起動]
       │
       ▼
[Rust] GET https://api.github.com/repos/kakeru-ikeda/Scrobcord/releases/latest
       User-Agent: Scrobcord/{version}
       │
       ├── 404 (リリース未公開) → available: false で返却
       ├── ネットワークエラー   → Err(String) → フロントエンドで握りつぶし
       └── 200 OK
             │
             ▼
            tag_name の v プレフィックスを除去 → semver 比較
             │
             ├── current >= latest → available: false で返却
             └── current <  latest → available: true, release_url で返却
                         │
                         ▼
            [フロントエンド] UpdateBanner を表示
                         │
                         ├── [ダウンロードページへ] → open_release_url → ブラウザで開く
                         └── [✕] → バナー非表示（セッション中のみ）
```

---

### Rust コマンド

**`commands/updater.rs`**

```rust
pub struct UpdateInfo {
    pub available: bool,
    pub latest_version: String,
    pub current_version: String,  // env!("CARGO_PKG_VERSION")
    pub release_url: String,
}

// GitHub API でアップデートを確認（ネットワークエラー時は available: false）
check_for_updates() -> Result<UpdateInfo, String>

// GitHub ドメインのみ許可して URL をブラウザで開く（SSRF 対策）
open_release_url(url: String) -> Result<(), String>
```

**セマンティックバージョン比較:**  
プレリリースサフィックス（例: `"0.2.0-beta"`）は数値部分のみで比較。`(major, minor, patch)` のタプル大小比較。

---

### フロントエンド

| ファイル                      | 役割                                                                        |
| ----------------------------- | --------------------------------------------------------------------------- |
| `lib/tauriInvoke.ts`          | `UpdateInfo` 型・`checkForUpdates()` / `openReleaseUrl(url)` ラッパー       |
| `hooks/useUpdateCheck.ts`     | マウント時に `checkForUpdates()` を呼び、dismiss 状態を管理                 |
| `components/UpdateBanner.tsx` | アップデートバナー UI（バージョン表示 + ダウンロードボタン + 閉じるボタン） |
| `pages/Dashboard.tsx`         | タイトルバー直下に `UpdateBanner` を条件表示                                |

---

### 追加する Tauri コマンド

```rust
check_for_updates() -> Result<UpdateInfo, String>
open_release_url(url: String) -> Result<(), String>
```
