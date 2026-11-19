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

`LASTFM_API_KEY` は署名サービス scrobsign に登録されたアプリ `scrobcord` の api_key です。秘密情報ではなく、値は `src-tauri/.env.example` にあります。この値と異なる api_key でビルドすると、ログイン時に「api_key が一致しません」というエラーになります。

**Discord Application**

1. [Discord Developer Portal](https://discord.com/developers/applications) で **New Application** を作成する
2. **Application ID** をコピーする
3. **Rich Presence → Art Assets** で `lastfm_icon` 等の画像をアップロードする（省略可）

`src-tauri/.env` ファイルを作成し、以下を記入する：

```env
LASTFM_API_KEY=3c209aa3012b54652e6cbe6be58e1075
DISCORD_APP_ID=your_application_id_here
```

### Last.fm の署名について

Scrobcord は Last.fm の api_secret を持ちません。認証(`auth.getToken`、`auth.getSession`)の署名は署名サービス scrobsign(`https://scrobsign.sukisuki.club`)から受け取ります。署名サービスは署名に必要なパラメータ(認証の token。scrobble するアプリでは session key(sk)と曲のメタデータ)を受け取りますが、保存せず、ログにも値を残しません。再生履歴などの読み取りは署名サービスを通さず Last.fm に直接問い合わせます。旧版(v0.0.4 以前)から更新すると Last.fm のキーが変わるため、1 回の再ログインが必要です。

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
