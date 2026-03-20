# Scrobcord

**Last.fm scrobble → Discord Rich Presence** に特化したクロスプラットフォームデスクトップアプリ。

Last.fm で再生中の楽曲を自動検出し、Discord のプロフィールにアルバムアート・曲名・アーティスト・タイムスタンプをリアルタイムで表示します。Discord Bot は不要です。

---

## 機能

- Last.fm の現在再生中トラックを自動ポーリング
- Discord Rich Presence（IPC ローカル通信）へのリアルタイム送信
- アルバムアート・タイムスタンプ・Last.fm リンクボタン表示
- Details / State の表示フォーマットをカスタマイズ可能（`{track}` / `{artist}` / `{album}` プレースホルダー）
- OS キーチェーン（keyring）による認証情報の安全な保存
- システムトレイ常駐・ログイン時自動起動

---

## 動作環境

| OS      | 動作確認状況 |
| ------- | ------------ |
| Windows | ✅           |
| macOS   | ✅           |
| Linux   | ✅           |

---

## セットアップ

### アプリをインストールする

[Releases](../../releases) から最新版をダウンロードしてください。

| ファイル        | 対象          |
| --------------- | ------------- |
| `.exe` / `.msi` | Windows       |
| `.dmg`          | macOS         |
| `.AppImage`     | Linux         |
| `.deb`          | Ubuntu/Debian |

---

## 使い方

1. アプリを起動する
2. **設定 → Last.fm** タブで **「Last.fm でログイン」** をクリックしてブラウザ認証する
3. ダッシュボードに戻り、Last.fm・Discord が両方「接続済」になっていることを確認する
4. 音楽を再生すると Discord プロフィールに曲情報が表示される

---

## ビルド方法

### 必要なツール

- [Node.js](https://nodejs.org/) v20 以上
- [Rust](https://www.rust-lang.org/tools/install) (stable)
- [Tauri CLI の依存関係](https://tauri.app/start/prerequisites/)（Linux は WebKit2GTK 等が必要）

### 事前準備（自分でビルドする場合）

リリースビルドには Last.fm API キー・Discord Application ID があらかじめ埋め込まれています。自分でビルドする場合は以下を用意してください。

**Last.fm API キー**

1. [Last.fm API アカウント作成ページ](https://www.last.fm/api/account/create) でアプリを登録する
2. **API Key** と **Shared Secret** を取得する

**Discord Application**

1. [Discord Developer Portal](https://discord.com/developers/applications) で **New Application** を作成する
2. **Application ID** をコピーする
3. **Rich Presence → Art Assets** で `lastfm_icon` 等の画像をアップロードする（省略可）

リポジトリルートに `.env` ファイルを作成し、以下を記入する：

```env
LASTFM_API_KEY=your_api_key_here
LASTFM_API_SECRET=your_api_secret_here
DISCORD_APP_ID=your_application_id_here
```

### 手順

```bash
# 依存パッケージのインストール
npm install

# 開発モード起動
npm run tauri dev

# リリースビルド
npm run tauri build
```

---

## 技術スタック

| レイヤー       | 技術                            |
| -------------- | ------------------------------- |
| デスクトップ   | [Tauri 2.x](https://tauri.app/) |
| バックエンド   | Rust / tokio                    |
| フロントエンド | React 18 + TypeScript + Vite    |
| UI             | Tailwind CSS + shadcn/ui        |
| 状態管理       | Zustand                         |
| 設定永続化     | tauri-plugin-store              |
| 認証情報保存   | keyring（OS キーチェーン）      |

---

## 推奨 IDE

[VS Code](https://code.visualstudio.com/) + [Tauri 拡張](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

---

## ライセンス

MIT
