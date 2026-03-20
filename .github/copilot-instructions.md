---
applyTo: "**"
---

# Scrobcord — GitHub Copilot 作業指示

## 作業前に必ず確認するファイル

1. **[DESIGN.md](../DESIGN.md)** — アーキテクチャ・技術スタック・API フロー・データ構造・UI 設計の全仕様
2. **[TASKS.md](../TASKS.md)** — フェーズ別の実装タスク一覧とチェック状況

これらを参照せずにコードを生成・変更しないこと。

---

## プロジェクト概要

Scrobcord は **Last.fm scrobble → Discord Rich Presence** 送信に特化したクロスプラットフォームデスクトップアプリ。  
スタック: **Tauri 2.x / Rust (tokio) / React 18 + TypeScript + Vite / Tailwind CSS + shadcn/ui / Zustand**

---

## コーディング規約

### Rust (`src-tauri/`)

- すべての Tauri コマンドは `commands/` 配下のモジュールに定義し、`lib.rs` で `invoke_handler` に登録する
- サービスロジック（API 呼び出し・IPC 通信）は `services/` に実装し、コマンド層から呼び出す
- `AppState` は `Arc<Mutex<...>>` で保持し、`tauri::State` で受け取る
- `session_key` / `api_secret` は **必ず keyring crate 経由で OS キーチェーンに保存**し、ファイルや Store への平文書き込みを禁止する
- エラーは `Result<T, String>` で Tauri コマンド境界まで伝播させる
- 非同期処理は `tokio::spawn` を使用し、ポーリングループには停止用の `CancellationToken` を必ず持たせる

### TypeScript / React (`src/`)

- Tauri invoke は `lib/tauriInvoke.ts` の型付きラッパーを経由する（直接 `invoke` を呼ばない）
- グローバル状態は Zustand の `appStore.ts` に集約する
- Tauri イベント購読は各 hook（`hooks/`）内で行い、コンポーネントから直接 `listen` しない
- コンポーネントは `components/` に配置し、ページは `pages/` に配置する

---

## イベント仕様（Rust → React）

| イベント名               | ペイロード型                                    |
| ------------------------ | ----------------------------------------------- |
| `track-changed`          | `{ track: Track \| null }`                      |
| `discord-status-changed` | `{ connected: boolean, error?: string }`        |
| `lastfm-status-changed`  | `{ authenticated: boolean, username?: string }` |
| `polling-status-changed` | `{ running: boolean }`                          |

---

## セキュリティ要件

- `api_secret` / `session_key` を平文でファイル・ストアに保存しない
- ユーザー入力のフォーマット文字列（`{track}` 等）はホワイトリスト方式で置換し、任意コード実行を防ぐ
- Last.fm API 署名は MD5(パラメータ文字列 + api_secret) — DESIGN.md の仕様に従う

---

## タスク管理ルール

- 実装を開始する前に **TASKS.md** の該当タスクを確認する
- タスクを完了したら TASKS.md の対応チェックボックスを `[x]` に更新する
- DESIGN.md に記載のない仕様変更が必要な場合は、先に DESIGN.md を更新してからコードに反映する
