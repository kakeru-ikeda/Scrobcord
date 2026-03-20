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
  - [x] ログイン時起動 toggle
  - [x] トレイ最小化 toggle
  - [x] 言語選択（ja / en）
- [x] 設定変更を `save_settings` で即時保存（デバウンス付き）

---

## Phase 8 — トレイアイコン・スタートアップ

- [ ] トレイアイコン画像（PNG 32×32）を `src-tauri/icons/` に追加
- [ ] `main.rs` にトレイメニュー（表示 / 停止・再開 / 終了）を実装
- [ ] ウィンドウの閉じるボタンでトレイに最小化（`minimize_to_tray=true` 時）
- [ ] `tauri-plugin-autostart` を `start_on_login` 設定と連動
- [ ] アプリ起動時に自動的に `start_polling` を実行（設定読み込み後）

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
- [ ] i18n 対応（`language` 設定に応じて ja/en 切り替え）
