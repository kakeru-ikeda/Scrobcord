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
[ユーザーがブラウザで承認]
        │
        ▼
[UI] 「承認しました」ボタン押下
        │
        ▼
[Rust] GET auth.getSession (API Key + Token + MD5 署名)
        │  → session_key 取得
        ▼
[Rust] session_key を OS キーチェーンに保存 (keyring)
        │
        ▼
[イベント] lastfm-auth-success → UI 更新
```

- `session_key` は `user.getrecenttracks`（read-only）には不要
- `api_secret` も OS キーチェーンに保存し平文ファイルには書かない

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

---

## 設定項目（Settings 構造体）

```rust
pub struct Settings {
    // Last.fm
    pub lastfm_api_key: String,
    pub lastfm_api_secret: String,   // UI 入力後に keyring へ移動
    pub lastfm_username: String,     // 認証後に自動設定

    // Discord RPC
    pub discord_app_id: String,
    pub discord_enabled: bool,
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

**「Last.fm」** — API Key / API Secret 入力、`[Last.fm でログイン]`→ブラウザ起動、`[承認完了]`→セッション取得、ユーザー名表示、ログアウト

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
