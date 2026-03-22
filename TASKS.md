# Scrobcord 実装タスク

> 設計詳細は [DESIGN.md](DESIGN.md) を参照。
> 各タスク完了後はチェックボックスを `[x]` にする。

---

## Phase 1 — Tauri プロジェクト初期化・基本構成

- [x] `npm create tauri-app@latest` で Tauri 2.x + React + TypeScript + Vite テンプレート生成
- [x] `src-tauri/Cargo.toml` に DESIGN.md 記載の依存クレートをすべて追加
- [x] `tauri-plugin-store`, `tauri-plugin-shell`, `tauri-plugin-autostart`, `tauri-plugin-log` を `lib.rs` に登録
- [x] `tauri.conf.json` の `identifier`, `windows` (title / size / decorations), `bundle` を設定
- [x] `src-tauri/src/` に `commands/`, `services/`, `models/` ディレクトリと各 `mod.rs` を作成
- [x] `state.rs` に `AppState` 構造体（Arc\<Mutex\<...\>>）の骨格を定義
- [x] `npm run tauri dev` でウィンドウが起動することを確認

---

## Phase 2 — Last.fm API クライアント + 認証フロー

- [x] `models/track.rs` に `Track` 構造体を定義（artist / title / album / album_art_url / url / timestamp）
- [x] `models/settings.rs` に `Settings` 構造体を DESIGN.md の仕様どおり定義
- [x] `services/lastfm.rs` に `LastfmClient` を実装
  - [x] `get_token()` — `auth.getToken` API 呼び出し + MD5 署名生成
  - [x] `get_session(token)` — `auth.getSession` API 呼び出し + session_key 取得
  - [x] `get_now_playing(username)` — `user.getrecenttracks` (limit=1) 呼び出し・再生中トラック抽出
  - [ ] `get_track_info(artist, track)` — `track.getInfo` からアルバムアート URL 取得（オプション）
- [x] `commands/auth.rs` に Tauri コマンドを実装
  - [x] `lastfm_get_auth_token()` — トークン取得 + ブラウザで認証 URL を開く
  - [x] `lastfm_get_session(token)` — session_key 取得 + keyring 保存
  - [x] `lastfm_logout()` — keyring から session_key 削除・状態リセット
  - [x] `lastfm_get_auth_status()` — keyring に session_key があるか確認・ユーザー名返却
- [x] `api_secret` を keyring に保存する処理を `save_settings` フローに組み込む
- [x] 認証成功時に `lastfm-status-changed` イベントを emit
- [x] MD5 署名ロジックの単体テスト作成

---

## Phase 3 — Discord RPC IPC クライアント

- [x] `discord-rich-presence` crate または IPC 直接実装を選定・追加
  - [x] Windows: Named Pipe `\\.\pipe\discord-ipc-{0..9}` 接続試行ループ
  - [x] Mac/Linux: Unix Socket `$TMPDIR/discord-ipc-{0..9}` 接続試行ループ
- [x] `services/discord_rpc.rs` に `DiscordRpcClient` を実装
  - [x] `connect(app_id)` — handshake フレーム送信
  - [x] `set_activity(track)` — SET_ACTIVITY payload 組み立て（details / state / assets / timestamps / buttons）
  - [x] `clear_activity()` — アクティビティクリア
  - [x] `disconnect()` — ソケットクローズ
- [x] `models/status.rs` に `DiscordStatus` / `AuthStatus` を定義
- [x] `commands/discord.rs` に Tauri コマンドを実装
  - [x] `discord_connect()`
  - [x] `discord_disconnect()`
  - [x] `discord_get_status()`
- [x] 接続状態変化時に `discord-status-changed` イベントを emit
- [x] Discord が起動していない場合のエラーハンドリング（再試行なし・即時エラー通知）

---

## Phase 4 — ポーリングループ (tokio task)

- [x] `services/poller.rs` に `PollerService` を実装
  - [x] `tokio::spawn` でループタスクを起動
  - [x] `poll_interval_secs` 間隔で Last.fm `get_now_playing` を呼び出す
  - [x] 前回トラックと比較し変化があれば `discord_rpc.set_activity()` 呼び出し
  - [x] 再生停止（`nowplaying` フラグなし）時に `clear_activity()` 呼び出し
  - [x] `CancellationToken` で停止できるようにする
- [x] `commands/polling.rs` に Tauri コマンドを実装
  - [x] `start_polling(app_handle)`
  - [x] `stop_polling()`
- [x] トラック変化時に `track-changed` イベントを emit
- [x] ポーリング状態変化時に `polling-status-changed` イベントを emit
- [x] Last.fm API エラー（レート制限・ネットワーク断）をログ出力しループ継続

---

## Phase 5 — 設定保存・読み込み (tauri-plugin-store)

- [x] `commands/settings.rs` に Tauri コマンドを実装
  - [x] `get_settings()` — Store から読み込み（存在しなければデフォルト値）
  - [x] `save_settings(settings)` — Store へ書き込み + `api_secret` は keyring へ移動（Store には保存しない）
- [x] アプリ起動時に Store から設定を読み込み `AppState` に反映
- [x] デフォルト値: `poll_interval_secs=15`, `discord_enabled=true`, `rpc_show_album_art=true`, `rpc_show_timestamp=true`, `rpc_show_lastfm_button=true`, `language="ja"`
- [x] `Settings` の JSON シリアライズ・デシリアライズのテスト作成

---

## Phase 6 — React ダッシュボード UI

- [x] Tailwind CSS + shadcn/ui のセットアップ
- [x] Zustand `appStore.ts` に state（`nowPlaying`, `discordStatus`, `lastfmStatus`, `pollingStatus`）を定義
- [x] `lib/tauriInvoke.ts` に型安全な invoke ラッパー関数を定義
- [x] `useNowPlaying.ts` hook
  - [x] 起動時に `get_now_playing` invoke
  - [x] `track-changed` イベントをリッスンして store 更新
- [x] `useConnectionStatus.ts` hook
  - [x] `discord-status-changed` / `lastfm-status-changed` / `polling-status-changed` イベントをリッスン
- [x] `NowPlayingCard.tsx` — アルバムアート / 曲名 / アーティスト / アルバム表示
- [x] `ConnectionStatus.tsx` — Last.fm / Discord 接続状態インジケーター
- [x] `Dashboard.tsx` — NowPlayingCard + ConnectionStatus + 停止/設定ボタン
- [x] 未認証・未接続状態の空状態 UI

---

## Phase 7 — React 設定 UI + フォーマットプレビュー

- [x] タブ付き `Settings.tsx` レイアウト（Last.fm / Discord RPC / 一般）
- [x] `LastfmSettings.tsx`
  - [x] API Key / API Secret 入力欄（Secret は masked）
  - [x] `[Last.fm でログイン]` ボタン → `lastfm_get_auth_token` invoke + ブラウザ起動
  - [x] `[承認完了]` ボタン → `lastfm_get_session` invoke
  - [x] ユーザー名表示・ログアウトボタン
- [x] `DiscordSettings.tsx`
  - [x] Application ID 入力欄
  - [x] Details / State フォーマット入力欄（`{track}` `{artist}` `{album}` プレースホルダー説明付き）
  - [x] アルバムアート / タイムスタンプ / Last.fm ボタン toggle
  - [x] フォーマットプレビュー（現在のトラック情報で即時レンダリング）
- [x] `GeneralSettings.tsx`
  - [x] ポーリング間隔スライダー（5〜60s、ラベル付き）
  - [x] ログイン時起動 toggle - [x] Discord RPC 送信 toggle（`rpc_enabled`）— 分植運用時に片方のマシンでの二重 RPC 書き込みを防止 - [x] トレイ最小化 toggle
  - [x] 言語選択（ja / en）
- [x] 設定変更を `save_settings` で即時保存（デバウンス付き）

---

## Phase 8 — トレイアイコン・スタートアップ

- [x] トレイアイコン画像（PNG 32×32）を `src-tauri/icons/` に追加
- [x] `lib.rs` にトレイメニュー（表示 / 停止・再開 / 終了）を実装
- [x] ウィンドウの閉じるボタンでトレイに最小化（`minimize_to_tray=true` 時）
- [x] `tauri-plugin-autostart` を `start_on_login` 設定と連動
- [x] アプリ起動時に自動的に `start_polling` を実行（設定読み込み後）

---

## Phase 9 — GitHub Actions CI/CD + リリース確認

- [ ] `.github/workflows/build.yml` を DESIGN.md の仕様どおり作成
- [ ] `scripts/release.js`（または npm スクリプト）で tag push → Actions トリガーの動作確認
- [ ] Ubuntu / Windows / macOS の 3 OS ビルドが成功することを確認
- [ ] Windows 自己署名証明書ステップの動作確認
- [ ] バージョン同期スクリプト（`package.json` / `tauri.conf.json` / `Cargo.toml`）が正しく動作することを確認
- [ ] GitHub Releases Draft に `.exe` / `.msi` / `.dmg` / `.AppImage` / `.deb` が揃うことを確認
- [ ] `README.md` にインストール手順・Discord Application 作成手順・Last.fm API Key 取得手順を記載

---

## 横断的タスク（随時）

- [ ] `tauri-plugin-log` でログレベル設定（debug ビルドは DEBUG、release は INFO）
- [ ] 全 Tauri コマンドのエラーを `String` ではなく構造化エラー型に統一（任意）
- [x] i18n 対応（`language` 設定に応じて ja/en 切り替え）

---

## Phase 10 — Scrobble 履歴表示（ページネーション）

> 詳細設計は [DESIGN.md](DESIGN.md) の「Phase 10」セクションを参照。

### Rust バックエンド

- [x] `models/track.rs` に `ScrobbledTrack` 構造体を追加（`now_playing: bool`, `timestamp: Option<i64>`）
- [x] `models/track.rs` に `RecentTracksPage` 構造体を追加（`tracks`, `page`, `per_page`, `total_pages`, `total_tracks`）
- [x] `services/lastfm.rs` に `get_recent_tracks(username, page, limit)` を実装
  - [x] `user.getRecentTracks` API 呼び出し（`page` / `limit` パラメータ付き）
  - [x] `recenttracks.@attr` からページネーション情報（文字列→数値）をパース
  - [x] `track` が配列/単一オブジェクト両ケースをハンドル
  - [x] nowplaying トラックの `timestamp = None` / `now_playing = true` を正しくセット
- [x] `commands/history.rs` を新規作成し `get_recent_tracks(page, limit)` コマンドを実装
  - [x] `AppState` から `lastfm_username` を取得、空の場合はエラー返却
- [x] `commands/mod.rs` に `pub mod history;` を追加
- [x] `lib.rs` の `invoke_handler` に `commands::history::get_recent_tracks` を登録

### TypeScript / React フロントエンド

- [x] `lib/tauriInvoke.ts` に `ScrobbledTrack` / `RecentTracksPage` 型を追加
- [x] `lib/tauriInvoke.ts` に `getRecentTracks(page, limit)` invoke ラッパーを追加
- [x] `lib/utils.ts` に `formatRelativeTime(unixSec: number): string` ユーティリティを追加
- [x] `hooks/useScrobbleHistory.ts` を新規作成
  - [x] `fetchPage(p)` 関数の実装（ローディング / エラー状態管理）
  - [x] `useEffect` で認証状態変化時に page=1 を自動ロード
  - [x] `track-changed` イベント購読で page=1 時に自動リフレッシュ
- [x] `components/ScrobbleHistory.tsx` を新規作成
  - [x] トラック行コンポーネント（アルバムアート 32px / 曲名+アーティスト / 相対時刻）
  - [x] nowplaying 行のバッジ表示（緑色「いま再生中」）
  - [x] overflow-y-auto のスクロール可能リスト
  - [x] ページネーションバー（前/次ボタン + ページ数表示）
  - [x] ローディング中のスケルトン表示
  - [x] エラー表示 + 再試行ボタン
- [x] `pages/Dashboard.tsx` のレイアウト変更
  - [x] NowPlayingCard を `shrink-0` に変更
  - [x] `ScrobbleHistory` を `flex-1 min-h-0` で配置（NowPlayingCard の下）

---

## Phase 11 — オンラインアップデート確認

> 詳細設計は [DESIGN.md](DESIGN.md) の「Phase 11」セクションを参照。

### Rust バックエンド

- [x] `commands/updater.rs` を新規作成
  - [x] `UpdateInfo` 構造体（`available`, `latest_version`, `current_version`, `release_url`）
  - [x] `check_for_updates()` — GitHub Releases API 呼び出し・semver 比較
  - [x] `open_release_url(url)` — GitHub ドメイン検証後にブラウザで開く（SSRF 対策）
  - [x] `is_newer_version()` の単体テスト
- [x] `commands/mod.rs` に `pub mod updater;` を追加
- [x] `lib.rs` の `invoke_handler` に `check_for_updates` / `open_release_url` を登録

### TypeScript / React フロントエンド

- [x] `lib/tauriInvoke.ts` に `UpdateInfo` 型・`checkForUpdates()` / `openReleaseUrl()` ラッパーを追加
- [x] `hooks/useUpdateCheck.ts` を新規作成
  - [x] マウント時に `checkForUpdates()` を呼び出し
  - [x] dismiss 状態をローカル state で管理
- [x] `components/UpdateBanner.tsx` を新規作成
  - [x] バージョン表示・「ダウンロードページへ」ボタン・閉じるボタン
- [x] `pages/Dashboard.tsx` のタイトルバー直下に `UpdateBanner` を条件表示
- [x] i18n キー追加（`update.available` / `update.openReleasePage` / `update.dismiss`）
