# Scrobcord の Last.fm 署名を scrobsign に移す

- 正本の設計書: `/home/server/github/scrobsign/docs/superpowers/specs/2026-11-19-scrobsign-design.md`(5.4、5.5、5.6、5.11、5.13、7、8.2、9.5)
- ブランチ: `kakeru-ikeda/scrobsign-signing`(main から分岐済み)
- PR タイトル: `feat: Last.fm の署名を署名サービス scrobsign に移す`(配布物が変わるので `feat:`)

## 未解決の質問

なし。判断が必要だった点は「Design notes」に決定として書いた。

## 共通の検証コマンド(`--verify` に渡す)

```sh
cd src-tauri && cargo test --lib --offline && cargo clippy --lib --offline -- -D warnings && cd .. && npm run build
```

- 2026-11-19 にこの環境で確認した(下の事前作業を済ませた状態)。main の時点では、`cargo test --lib` がコンパイルエラーで失敗する(`src-tauri/src/commands/settings.rs` 202 行目の struct literal に `start_minimized` が無い。E0063)。Task 1 の最初の手順でこれを直す。
- `cargo clippy --lib -- -D warnings` は main の時点で警告 0 件。`--all-targets` は確認していないので使わない。
- `npm run build` は `tsc && vite build`。main の時点で成功する。
- どの cargo コマンドを実行しても、`src-tauri/Cargo.lock` の `name = "scrobcord"` の `version = "0.0.3"` が `"0.0.4"` に書き換わる(lock ファイルが古いため)。この 1 行の差分は想定内で、Task 1 の成果に含めてよい。

## オーケストレーターの事前作業(Task 1 を委譲する前に 1 回だけ)

ワーカーのサンドボックスにはネットワークが無いので、次の 3 つを先に済ませる。

1. **OpenSSL の開発用ファイル**(必須)。`reqwest` の既定の TLS(`native-tls` → `openssl-sys`)が `openssl.pc` とヘッダーを要求する。この環境には `libssl-dev` が入っておらず、`sudo` にはパスワードが要る。
   - 推奨: ユーザーが `sudo apt install libssl-dev` を実行する。共通の検証コマンドは、この状態を前提にしている。
   - 代替(root なしの場合): `.deb` を展開して恒久的な場所に置き、検証コマンドの先頭に環境変数を付ける。

     ```sh
     D=$HOME/.local/opt/libssl-dev && mkdir -p "$D/deb" && cd "$D/deb" && apt download libssl-dev && dpkg-deb -x libssl-dev_*.deb "$D/root"
     cp -n "$D"/root/usr/include/x86_64-linux-gnu/openssl/* "$D/root/usr/include/openssl/"
     ln -sf /usr/lib/x86_64-linux-gnu/libssl.so.3 "$D/root/usr/lib/x86_64-linux-gnu/libssl.so"
     ln -sf /usr/lib/x86_64-linux-gnu/libcrypto.so.3 "$D/root/usr/lib/x86_64-linux-gnu/libcrypto.so"
     ```

     この場合の検証コマンドは次のとおり。この手順は scratchpad で実際に試し、`cargo test --lib` と `cargo clippy --lib` が通ることを確かめた。

     ```sh
     export OPENSSL_INCLUDE_DIR=$HOME/.local/opt/libssl-dev/root/usr/include OPENSSL_LIB_DIR=$HOME/.local/opt/libssl-dev/root/usr/lib/x86_64-linux-gnu && cd src-tauri && cargo test --lib --offline && cargo clippy --lib --offline -- -D warnings && cd .. && npm run build
     ```
2. `cd src-tauri && cargo fetch`。crate を `~/.cargo/registry` に取得する。この環境では取得済み。
3. リポジトリのルートで `npm ci`。`node_modules` を作る。

## 人間の作業(この計画のタスクには含めない)

順序どおりに行う。

1. `sudo apt install libssl-dev`(上の事前作業 1。代替手順を使うなら不要)。
2. Scrobcord リポの GitHub secret `LASTFM_API_KEY` を、新しいキーの api_key(`3c209aa3012b54652e6cbe6be58e1075`)に差し替える。**タグを打つ前に**行う。差し替えないまま配布すると、ログイン時に「api_key が一致しません」のエラーになる(Task 2 の検査)。
3. PR(`feat:`)をレビューしてマージする。
4. `scripts/release.mjs` でバージョンを上げ、タグを push する。`build.yml` が作った draft のリリースを publish する(設計書 7.4 の 15)。
5. GitHub secret `LASTFM_API_SECRET` を削除する(設計書 7.4 の 16)。
6. publish から猶予期間(既定 7 日)を置いて、旧キーを失効させる(設計書 7.5 の 17)。
7. 設計書 7.6 の実機確認: 認証、now playing の表示、旧版からの更新後に再ログインの案内が出ること。

---

# 計画

## Goal

Scrobcord から Last.fm の api_secret を取り除き、署名が必要な呼び出し(`auth.getToken`、`auth.getSession`)の署名を、署名サービス scrobsign(`POST https://scrobsign.sukisuki.club/v1/scrobcord/sign`)から受け取る。既存ユーザーは旧キーで発行された session key を持っているので、それを検出して未認証の状態に戻し、「Last.fm のキーが変わったため、再ログインが必要です」と表示する。CI では、`LASTFM_API_SECRET` がソースに残っていないことを検査する。

## Non-goals

- scrobble(`track.scrobble`、`track.updateNowPlaying`)の実装。Scrobcord は読み取り(`user.getRecentTracks`)しかしない。
- 読み取り系を署名サービス経由にすること。読み取りは、ビルド時に埋め込む `LASTFM_API_KEY`(新しいキーの api_key。秘密ではない)をそのまま使う。
- GitHub secret の更新、旧キーの失効、タグとリリース(「人間の作業」を参照)。
- `DESIGN.md` と `TASKS.md`(過去の設計メモ)の書き換え。
- HTTP のモック用のクレートを入れること。
- reqwest の TLS バックエンドを rustls に切り替えること(配布物の挙動が変わるため。OpenSSL は事前作業で解決する)。

## Design notes

- **新しいモジュールは 2 つ。どちらも Tauri に依存しない。**
  - `src-tauri/src/services/scrobsign.rs`: 署名サービスとのやり取りを受け持つ。定数、`Signature`、`SignError`、リクエストの本文の組み立て、応答の解析を置く。HTTP の呼び出しは Task 2 で足す。
  - `src-tauri/src/services/auth_flow.rs`: Last.fm の認証フローの判断を受け持つ。Last.fm のエラーの解析、`AuthError` とその分類、token ごとの署名の使い回し(`TokenSignatureCache`)、api_key の一致判定を置く。
  - HTTP のモックは使わない。判断はすべて純粋な関数と同期の状態機械にまとめ、`cargo test --lib` で確かめる。HTTP を呼ぶ部分は薄くし、「本文を読んで純粋な関数に渡す」だけにする。
- **dead_code の扱い。** `mod services` は `lib.rs` の中で非公開なので、配線する前のモジュールは、テストから使っていても lib ターゲットで dead_code の警告になり、`-D warnings` で落ちる。Task 1 では 2 つのファイルの先頭に `#![allow(dead_code)]` を付ける。外す時期は次のとおり。
  - `scrobsign.rs`: Task 2 で外す。
  - `auth_flow.rs`: `evaluate_stored_session` を使い始める Task 3 で外す。
- **Last.fm のエラーは本文で判定する。** Last.fm はエラーを HTTP 400 や 403 で返し、本文に `{"error": N, "message": "..."}` を入れる。HTTP ステータスで先に弾くと、エラーコードが分からなくなる。scrobsign の確認機能が一度これでバグった。署名サービスのエラーは、HTTP ステータスと `error.code` で判定してよい(設計書 5.13)。
- **エラー 14 の判定を型で行う。** 現在の `commands/auth.rs` 158 行目は `e.contains("14")` で判定しているが、`get_session` はコードを文字列に含めていないので、この判定は当たっていない。型付きの `AuthError::Lastfm(LastfmApiError { code, .. })` に置き換え、`String` に変えるのは Tauri へ返す境界でだけにする。
- **token ごとに署名を 1 回だけ取る。** `auth.getSession` の署名は token だけで決まる。ポーリングのタスクは `TokenSignatureCache` を持ち、3 秒ごとの呼び出しでは同じ署名を使い回す。エラー 13 を受けたときだけ署名を捨て、1 回だけ取り直す。2 回目の 13 では止める。手動のフォールバック(`lastfm_get_session`)も同じ署名を使えるように、ポーリングのタスクは取得した署名を `AppStateInner.pending_auth.signature` にも写す。
- **認証 URL と、保存する api_key は、署名サービスが返した値を使う。** さらにログインの時点で、署名サービスの api_key と埋め込みの api_key が一致しなければ、その場で `AuthError::KeyMismatch` にする。GitHub secret の差し替えを忘れて配布した場合に、毎回の起動で「キーが変わった」と出て再ログインを繰り返すことを防ぎ、原因をすぐ分かるようにするためである。
- **発行時の api_key は `Settings` に入れない。** 同じ Store(`settings.json`)の別のキー `lastfm_session_api_key` に、文字列として保存する。
  - `Settings` に入れない理由: フロントの `save_settings` は TS の `Settings` 型のオブジェクトを送り返す。この型に無いフィールドは `#[serde(default)]` で空になり、次の起動で必ず不一致になる。
  - 保存する箇所: `complete_auth`。消す箇所: `lastfm_logout`、`reset_saved_data`。
- **起動時の判定は `evaluate_stored_session(has_session_key, stored_api_key, embedded_api_key)` で行う。**
  - 保存値が無い場合も「キーが変わった」として扱う。v0.0.4 以前のユーザーは保存値を持っていないので、**ここが移行の要**である。
  - 埋め込みの api_key が空のときは、判定を Valid にしない。
- **不一致を検出しても、keyring の session key は消さない。** 起動するたびに再び検出して案内を出し続けるためである。次のどちらかの時点で上書きまたは削除される。
  - 再ログインした時点で、`complete_auth` が上書きする。
  - ログアウトかリセットをした時点で、削除する。
  - 旧キーの session key は新しいキーでは使えないので、残しても害は無い。
- **案内は `AuthStatus.notice` で運ぶ。**
  - Rust 側: `Option<String>`。`#[serde(default, skip_serializing_if = "Option::is_none")]` を付ける。値は機械向けのコード `"key_changed"` にする。
  - フロント側: i18n の `lastfm.keyChanged` で表示する。ja は「Last.fm のキーが変わったため、再ログインが必要です」。
- **ポーリングが止まった理由は、イベントで UI に伝える。** `lastfm-auth-polling` のペイロードを `{ polling: false, error?: string }` に広げる。設計書 5.11 の各エラー(署名サービスが使えない、サービス側の問題、トークンの期限切れ)は、ここから画面に出る。
- **5.11 の表の当てはめ(認証フロー)**
  - 再試行する(ポーリングを続ける): 署名サービスへの通信失敗、429、5xx(`app_misconfigured` を含む)、Last.fm への通信失敗、エラー 14、その他の Last.fm のエラー。
  - 止めて「署名サービスが使えません」と表示する: 403 `app_disabled`、403 `method_forbidden`、404 `unknown_app`。
  - 署名を 1 回だけ取り直す: エラー 13。
  - 止めて「サービス側の問題」と表示する: エラー 10 と 26。
  - 止めて再ログインを促す: エラー 4 と 15(token の期限切れなど)。
  - `auth.getToken` では、エラー 13 のときに署名を取り直して 1 回だけ再試行する。それ以外のエラーは、利用者向けのメッセージにして返す。
- **秘密の値をログやエラー文に出さない。**
  - token、api_sig、sk を、ログ、エラー文、`Debug` の出力に含めない。
  - `Signature` の `Debug` は手書きにして、api_sig を `<redacted>` と表示する。
  - reqwest のエラーは `e.without_url().to_string()` にする。Last.fm への GET の URL には token と api_sig が載るためである。
  - 既存のコードにある `format!("... {resp}")`(応答の全体を埋め込む書き方)を、新しいコードでは使わない。
- **署名サービスへの HTTP**
  - `POST {SCROBSIGN_URL}/v1/scrobcord/sign`。本文は `{"method":"<name>","params":{...}}` で、`Content-Type: application/json` を付ける。
  - `params` に `api_key`、`api_sig`、`method`、`format`、`callback` を入れない(設計書 5.4)。
  - タイムアウトは接続 10 秒、全体 15 秒。`User-Agent` は `Scrobcord/<CARGO_PKG_VERSION>`。
- **CI の検査**
  - `scripts/check-no-lastfm-secret.sh` を追加し、`git grep --untracked` で `src-tauri/` と `.github/` を調べる。
  - 検索パターンは `'LASTFM_API_''SECRET'` のように連結して組み立てる。こうしないと、検査そのものが自分にマッチしてしまう。
  - `build.yml` には ubuntu だけで動く独立の job を足し、`shell: bash` でこのスクリプトを呼ぶ。matrix に windows があり、既定のシェルが pwsh だからである。
  - あわせて、build job の ubuntu の行に `cargo test --lib` のステップを足す。
- **採用しなかった案**
  - トレイトで HTTP を差し替える案。`LastfmClient` を `AppState` に入れて Clone しているので、型引数が全体に広がる。判断を純粋な関数に寄せれば、モックは要らない。
  - 署名の応答の api_key だけを使い、`LASTFM_API_KEY` の埋め込みをやめる案。読み取り系は、ログイン前やオフラインでも署名サービスに依存せずに動かしたい。設計書 8.2 も埋め込みを維持している。
  - 不一致を検出したときに keyring の session key を消す案。案内が 1 回で消えてしまう。

---

### Task 1: 既存テストを直し、署名サービスと認証フローの純粋なロジックを追加する

- 設計書: 5.4、5.11、5.13、8.2、9.5
- 依存タスク: なし
- Files to touch:
  - `src-tauri/src/commands/settings.rs`(テストの struct literal だけ)
  - `src-tauri/src/services/mod.rs`
  - `src-tauri/src/services/scrobsign.rs`(新規)
  - `src-tauri/src/services/auth_flow.rs`(新規)
  - `src-tauri/Cargo.lock`(cargo が自動で書き換える `scrobcord` の version の 1 行だけ)
- Files NOT to touch: `src-tauri/src/services/lastfm.rs`、`src-tauri/src/commands/auth.rs`、`src-tauri/src/state.rs`、`src-tauri/src/lib.rs`、`src-tauri/Cargo.toml`、`src/**`、`.github/**`
- New dependencies: none(`serde`、`serde_json`、`tokio` は既存)
- Steps:
  1. **既存テストの修正。** `src-tauri/src/commands/settings.rs` の `settings_json_round_trip` テスト(202 行目付近)の `Settings { ... }` に `start_minimized: false,` を足す。これで main のコンパイルエラー(E0063)が直る。
  2. **モジュールの登録。** `src-tauri/src/services/mod.rs` に `pub mod auth_flow;` と `pub mod scrobsign;` を足す。
  3. **`scrobsign.rs` を作る。** 先頭に `#![allow(dead_code)] // Task 2 で配線したら外す` を置く。中身は次のとおり。
     - 定数:
       - `pub const SCROBSIGN_URL: &str = "https://scrobsign.sukisuki.club";`
       - `pub const SCROBSIGN_APP_ID: &str = "scrobcord";`
       - `pub const RESERVED_PARAMS: [&str; 5] = ["api_key", "api_sig", "method", "format", "callback"];`
     - `pub fn sign_endpoint() -> String`: `"https://scrobsign.sukisuki.club/v1/scrobcord/sign"` を返す。
     - `#[derive(Clone, PartialEq, Eq)] pub struct Signature { pub api_key: String, pub api_sig: String }`
       - `impl std::fmt::Debug` を手書きする。出力は `Signature { api_key: "<値>", api_sig: "<redacted>" }` の形にする。
     - `#[derive(Debug, Clone, PartialEq, Eq)] pub enum SignError`。バリアントは次のとおり。
       - `Transport(String)`: 接続の失敗やタイムアウト。
       - `RateLimited { retry_after_secs: Option<u64> }`: 429。
       - `ServerError { status: u16, code: Option<String> }`: 5xx。
       - `Unavailable { status: u16, code: String }`: 403 の `app_disabled` か `method_forbidden`、または 404 の `unknown_app`。
       - `Rejected { status: u16, code: Option<String> }`: その他の 4xx。
       - `InvalidInput(String)`: 予約されたパラメータ名。値ではなく名前を入れる。
       - `BadResponse(String)`: 200 なのに形式が不正。値は入れない。
     - `impl SignError`:
       - `pub fn is_retryable(&self) -> bool`: `Transport`、`RateLimited`、`ServerError` のときだけ true。
       - `pub fn user_message(&self) -> String`(日本語)。**各バリアントのフィールドをすべて文言に埋め込む。** derive した `Debug` からの読み取りは dead_code の判定で数えられないので、どこかで読まないと、Task 2 で allow を外した時点で「field is never read」になって `-D warnings` で落ちるためである。文言は次のとおり。
         - `Unavailable { status, code }` → `"署名サービスが使えません（HTTP {status} {code}）。アプリを最新版に更新するか、時間をおいて再度お試しください"`
         - `Transport(d)` → `"署名サービスに接続できません（{d}）。しばらくしてから再度お試しください"`
         - `RateLimited { retry_after_secs }` → `"署名サービスが混み合っています（{n} 秒後に再試行できます）"`。None のときは括弧の中を `"しばらくしてから再度お試しください"` にする。
         - `ServerError { status, code }` → `"署名サービスで一時的な障害が起きています（HTTP {status} {code か "-"}）"`
         - `Rejected { status, code }` → `"署名サービスがリクエストを拒否しました（HTTP {status} {code か "-"}）"`
         - `InvalidInput(name)` → `"署名サービスへ送れないパラメータです（{name}）"`
         - `BadResponse(d)` → `"署名サービスの応答が不正です（{d}）"`
       - 新しいコードのコメントや文字列に、`api_secret` と `LASTFM_API_SECRET` の語を書かない(Task 2 以降の grep 検査に引っかかるため)。
     - `impl std::fmt::Display for SignError`: `user_message` と同じ文字列を返す。
     - `pub fn build_sign_request_body(method: &str, params: &std::collections::BTreeMap<String, String>) -> Result<String, SignError>`
       - `params` のキーが `RESERVED_PARAMS` に含まれていれば、`Err(SignError::InvalidInput(<キー名>))` を返す。
       - それ以外は、`serde_json::json!({"method": method, "params": params})` を文字列にして返す。`params` が空なら `"params":{}` になる。
     - `pub fn parse_sign_response(status: u16, retry_after: Option<&str>, body: &str) -> Result<Signature, SignError>`
       - 200: 本文を JSON として読み、`api_key` と `api_sig` が文字列で、どちらも `^[0-9a-f]{32}$` に合えば `Ok` を返す。合わなければ `BadResponse` にする。メッセージに入れるのは `"missing api_sig"` のような要約だけで、値は入れない。
       - 429: `RateLimited` を返す。`retry_after_secs` は `retry_after` を `u64` として読んだ値で、読めなければ None。
       - 500 から 599: `ServerError` を返す。`code` は本文の `error.code`。読めなければ None。
       - 403 で `error.code` が `app_disabled` か `method_forbidden` のとき、または 404 で `error.code` が `unknown_app` のとき: `Unavailable` を返す。
       - その他の 400 から 499: `Rejected` を返す。
       - それ以外のステータス: `BadResponse(format!("unexpected status {status}"))` を返す。
  4. **`auth_flow.rs` を作る。** 先頭に `#![allow(dead_code)] // Task 3 で外す` を置く。中身は次のとおり。
     - Last.fm のエラーコードの定数:
       - `pub const LFM_INVALID_TOKEN: i64 = 4;`
       - `pub const LFM_INVALID_API_KEY: i64 = 10;`
       - `pub const LFM_INVALID_SIGNATURE: i64 = 13;`
       - `pub const LFM_UNAUTHORIZED_TOKEN: i64 = 14;`
       - `pub const LFM_TOKEN_EXPIRED: i64 = 15;`
       - `pub const LFM_SUSPENDED_API_KEY: i64 = 26;`
     - `#[derive(Debug, Clone, PartialEq, Eq)] pub struct LastfmApiError { pub code: i64, pub message: String }`
     - `pub fn parse_lastfm_error(body: &serde_json::Value) -> Option<LastfmApiError>`
       - `body["error"]` が整数(または整数を表す文字列)であれば `Some` を返す。`message` が無ければ空文字にする。
       - HTTP ステータスは引数に取らない。
     - `#[derive(Debug, Clone, PartialEq, Eq)] pub struct AuthToken { pub token: String, pub api_key: String }`
     - `#[derive(Debug, Clone, PartialEq, Eq)] pub enum AuthError`。バリアントは次のとおり。
       - `Sign(crate::services::scrobsign::SignError)`
       - `Lastfm(LastfmApiError)`
       - `Transport(String)`
       - `BadResponse(String)`
       - `KeyMismatch`
     - `impl AuthError { pub fn user_message(&self) -> String }` と、同じ文字列を返す `Display`。文言は次のとおり。
       - `Sign(e)` → `e.user_message()`
       - Last.fm の 10 と 26 → `"Last.fm のサービス側の問題で認証できません（error {code}）"`
       - 13 → `"Last.fm の署名が無効です（error 13）。しばらくしてから再度お試しください"`
       - 4 と 15 → `"認証トークンが無効か期限切れです。もう一度ログインしてください"`
       - その他の Last.fm のエラー → `"Last.fm エラー {code}: {message}"`
       - `Transport(d)` → `"Last.fm に接続できません: {d}"`
       - `BadResponse(d)` → `"Last.fm の応答を解釈できません（{d}）"`
       - `KeyMismatch` → `"署名サービスの api_key とアプリに埋め込まれた api_key が一致しません。ビルド設定（LASTFM_API_KEY）を確認してください"`
     - `pub fn parse_token_response(body: &str) -> Result<String, AuthError>`
       - 本文が JSON でなければ `BadResponse(format!("non-JSON body ({} bytes)", body.len()))` を返す。本文そのものは入れない。
       - `parse_lastfm_error` が `Some` を返せば `Lastfm` にする。
       - `token` が文字列なら `Ok` を返す。無ければ `BadResponse("token missing")` にする。
     - `pub fn parse_session_response(body: &str) -> Result<(String, String), AuthError>`
       - 返り値は `(session_key, username)`。`session.key` と `session.name` を読む。
       - エラーの扱いは `parse_token_response` と同じにする。
       - session key を含むエラー文は作らない。
     - `pub fn should_resign_token_request(err: &AuthError, attempt: u32) -> bool`: `Lastfm` の code が 13 で、`attempt == 0` のときだけ true。
     - `pub fn check_signer_api_key(signer_api_key: &str, embedded_api_key: &str) -> Result<(), AuthError>`: 2 つが一致しなければ `Err(KeyMismatch)` を返す。
     - `#[derive(Debug, Clone, PartialEq, Eq)] pub enum PollAction { Retry, RetryWithFreshSignature, Stop(String) }`
     - `#[derive(Debug, Default)] pub struct TokenSignatureCache { signature: Option<Signature>, resigned: bool }`。メソッドは次のとおり。
       - `pub fn new() -> Self`
       - `pub fn get(&self) -> Option<&Signature>`
       - `pub fn store(&mut self, sig: Signature)`
       - `pub fn on_error(&mut self, err: &AuthError) -> PollAction`。規則は次のとおり。
         - 14 → `Retry`。署名は保持する。
         - 13 で `resigned == false` → `resigned = true` にし、`signature = None` にして `RetryWithFreshSignature` を返す。
         - 13 で `resigned == true` → `Stop(err.user_message())`
         - 10、26、4、15 → `Stop(err.user_message())`
         - その他の Last.fm のエラー → `Retry`
         - `Sign(e)` で `e.is_retryable()` が true → `Retry`
         - その他の `Sign(e)` → `Stop(err.user_message())`
         - `Transport` と `BadResponse` → `Retry`
         - `KeyMismatch` → `Stop(err.user_message())`
     - `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum StoredSessionState { Valid, NotLoggedIn, KeyChanged }`
     - `pub fn evaluate_stored_session(has_session_key: bool, stored_api_key: Option<&str>, embedded_api_key: &str) -> StoredSessionState`
       - `!has_session_key` → `NotLoggedIn`
       - `embedded_api_key` が空でなく、かつ `stored_api_key == Some(embedded_api_key)` → `Valid`
       - それ以外(保存値が無い場合を含む) → `KeyChanged`
  5. **テストを先に書く(TDD)。** 各ファイルの末尾の `#[cfg(test)] mod tests` に置く。
     - 値はダミーにする。api_key は `"0123456789abcdef0123456789abcdef"`、api_sig は `"0000000000000000000000000000abcd"`、token は `"dummytoken"`、session key は `"dummysessionkey"` を使う。secret らしい文字列や、本物のキーは使わない。
     - `scrobsign.rs` のテスト:
       - `sign_endpoint()` の値。
       - `build_sign_request_body` が `auth.getToken`(空の params)と `auth.getSession`(token)で期待どおりの JSON を作ること。JSON として読み戻して比べる。
       - 予約名(`api_key`、`format`)が `InvalidInput` になること。
       - `parse_sign_response` の 200 の正常系。
       - 200 で `api_sig` が欠けているとき、32 文字の 16 進数でないとき、本文が JSON でないとき → `BadResponse`。エラー文にダミーの api_sig が含まれないこと。
       - 429 で `Retry-After: 30` のとき、ヘッダーが無いとき。
       - 503 の `app_misconfigured` → `ServerError` で、`is_retryable` が true になること。
       - 403 の `app_disabled`、403 の `method_forbidden`、404 の `unknown_app` → `Unavailable` で、`is_retryable` が false になること。
       - 400 の `invalid_params` → `Rejected`。
       - `format!("{:?}", sig)` に api_sig の値が含まれないこと。
     - `auth_flow.rs` のテスト:
       - `parse_lastfm_error`: `{"error":14,"message":"..."}`、`{"error":"13",...}`、error の無い本文。
       - `parse_token_response`: 成功、エラー 13、JSON でない本文。JSON でない本文のエラー文に、本文の中身が含まれないこと。
       - `parse_session_response`: 成功、エラー 14、`session.name` の欠落。
       - `should_resign_token_request` の真理値表。
       - `check_signer_api_key` の一致と不一致。
       - `evaluate_stored_session` の全組み合わせ。少なくとも、session 無し、保存値無し(旧版のユーザー)、保存値が不一致、保存値が一致、埋め込みが空。
       - `TokenSignatureCache` のシミュレーション。テスト内に「キャッシュが空なら fetch 回数を 1 増やしてダミーの署名を `store` し、続けて台本どおりの `AuthError` を `on_error` に渡す」というループを書き、次を確かめる。
         - エラー 14 が 100 回続いても fetch は 1 回。
         - 13 の次に 14 が続くと fetch は 2 回で、`RetryWithFreshSignature` が 1 回返る。
         - 13 が 2 回続くと 2 回目で `Stop`。
         - 署名サービスの `Transport` → `Retry`。
         - `Unavailable` → `Stop` で、メッセージに「署名サービスが使えません」が含まれる。
         - `SignError` の全バリアントで `user_message()` が空でないこと(フィールドが文言に入っていることの確認を兼ねる)。
         - 10 と 26 → `Stop` で、「サービス側の問題」が含まれる。
- Acceptance:
  - 共通の検証コマンドが成功する。`cargo test --lib` に上の新しいテストが含まれ、すべて通る。
  - `lastfm.rs` と `commands/auth.rs` の挙動は変わらない(このタスクでは配線しない)。
- Verify: `cd src-tauri && cargo test --lib --offline && cargo clippy --lib --offline -- -D warnings && cargo test --lib --offline scrobsign:: && cargo test --lib --offline auth_flow:: && cd .. && npm run build`

### Task 2: 署名サービスを呼ぶように認証フローを書き換え、api_secret と sign() を削除する

- 設計書: 5.4、5.11、8.2(`lastfm.rs` の項目すべて)
- 依存タスク: Task 1
- Files to touch:
  - `src-tauri/src/services/scrobsign.rs`
  - `src-tauri/src/services/lastfm.rs`
  - `src-tauri/src/commands/auth.rs`
  - `src-tauri/src/state.rs`
  - `src-tauri/src/lib.rs`(`AppStateInner` の初期化だけ)
  - `src-tauri/src/commands/settings.rs`(`pending_auth_token` の参照だけ)
  - `src-tauri/Cargo.toml`(`md5` の削除)
  - `src-tauri/Cargo.lock`(cargo による自動更新)
- Files NOT to touch: `src-tauri/src/services/auth_flow.rs` の公開 API(シグネチャを変えない)、`src-tauri/src/services/poller.rs`、`src-tauri/build.rs`、`src/**`、`.github/**`、`README.md`
- New dependencies: none(`md5 = "0.7"` を削除する)
- Steps:
  1. **`scrobsign.rs`: dead_code の allow を外し、HTTP の関数を足す。**
     - 先頭の `#![allow(dead_code)]` を外す。
     - `pub async fn request_signature(client: &reqwest::Client, method: &str, params: &BTreeMap<String, String>) -> Result<Signature, SignError>` を足す。処理は次のとおり。
       - `build_sign_request_body` で本文を作る。
       - `client.post(sign_endpoint()).header(reqwest::header::CONTENT_TYPE, "application/json").timeout(Duration::from_secs(15)).body(body).send().await` を呼ぶ。送信のエラーは `SignError::Transport(e.without_url().to_string())` にする。
       - ステータスと `Retry-After` ヘッダーを取り出し、本文を `text()` で読んで、`parse_sign_response` に渡す。
     - このファイルの中のどの項目も、使われずに残らないようにする。
  2. **`lastfm.rs`: secret と署名の処理を削除する。**
     - `EMBEDDED_API_SECRET`、`api_secret` フィールド、`fn sign`、既存の署名テスト 2 件(`test_sign_correct_order_and_hash`、`test_sign_keys_sorted_lexicographically`)、`use std::collections::BTreeMap` のうち不要になったものを削除する。
     - 9 行目のコメントは `// ビルド時に環境変数 LASTFM_API_KEY(新しいキーの api_key。秘密ではない)を埋め込む` に変える。
  3. **`LastfmClient::new` の HTTP クライアントを作り直す。** `reqwest::Client::builder().connect_timeout(Duration::from_secs(10)).user_agent(concat!("Scrobcord/", env!("CARGO_PKG_VERSION"))).build()` で作る。失敗したら `reqwest::Client::new()` にフォールバックする。
  4. **`LastfmClient` にメソッドを足す。**
     - `pub async fn request_signature(&self, method: &str, params: &BTreeMap<String, String>) -> Result<Signature, SignError>`: `scrobsign::request_signature(&self.client, ...)` を呼ぶだけ。
     - `pub fn embedded_api_key(&self) -> &str`: `&self.api_key` を返す。
  5. **`LastfmSession` に `pub api_key: String` を足す。** これは発行時の api_key で、`auth.getSession` の署名の応答から得た値を入れる。このタスクではまだ誰も読まないので、フィールドに `#[allow(dead_code)] // Task 3 で外す` を付ける(付けないと `-D warnings` で落ちる)。
  6. **`get_token` を書き換える。** シグネチャは `pub async fn get_token(&self) -> Result<AuthToken, AuthError>` にする。処理は次のとおり。
     - `attempt` が 0 と 1 の 2 回のループを回す。
     - 各回で、空の params で `"auth.getToken"` の署名を取る。失敗したら `AuthError::Sign` で返す。
     - `check_signer_api_key(&sig.api_key, &self.api_key)?` で一致を確かめる。
     - `GET API_ROOT?method=auth.getToken&api_key=<sig.api_key>&api_sig=<sig.api_sig>&format=json` を呼ぶ。送信のエラーは `AuthError::Transport(e.without_url().to_string())` にする。
     - **HTTP ステータスを見ずに**本文を `text()` で読み、`parse_token_response` に渡す。
     - エラーで `should_resign_token_request(&e, attempt)` が true なら、次の回へ進む。
     - 成功したら `AuthToken { token, api_key: sig.api_key }` を返す。
  7. **`get_session` を書き換える。** 名前とシグネチャを `pub async fn get_session_signed(&self, token: &str, sig: &Signature) -> Result<LastfmSession, AuthError>` にする。
     - `GET API_ROOT?method=auth.getSession&api_key=<sig.api_key>&token=<token>&api_sig=<sig.api_sig>&format=json` を呼ぶ。
     - 本文を `parse_session_response` に渡す。
     - 成功したら `LastfmSession { key, username, api_key: sig.api_key.clone() }` を返す。
     - 署名は取らない。呼び出し側が渡す。
  8. **`state.rs`: 保留中の認証を構造体にする。**
     - `pending_auth_token: Option<String>` を `pending_auth: Option<PendingAuth>` に変える。
     - `pub struct PendingAuth { pub token: String, pub signature: Option<crate::services::scrobsign::Signature> }` を追加する。
     - `lib.rs` 67 行目と `commands/settings.rs` 154 行目の参照を合わせて直す。
  9. **`commands/auth.rs` の `lastfm_get_auth_token`。**
     - 埋め込みの api_key が空なら拒否する処理(91 から 96 行目)は残す。ただし判定は `client.embedded_api_key().is_empty()` に替える(`embedded_api_key()` をこのタスクの中で使い、dead_code にしないため)。
     - `client.get_token().await.map_err(|e| e.user_message())?` で `AuthToken` を得る。
     - `pending_auth = Some(PendingAuth { token, signature: None })` を保存する。
     - 認証 URL には `auth_token.api_key` を使う。
  10. **`commands/auth.rs` のポーリングのループ。** 次の形に書き換える。
      - ループの外で `let mut cache = TokenSignatureCache::new();` と `let mut stop_message: Option<String> = None;` を用意する。
      - 各周期の流れ:
        1. `cache.get()` が None なら、`client.request_signature("auth.getSession", &{"token": token})` で署名を取る。
           - 成功したら `cache.store` し、`state_arc` の `pending_auth.signature` にも複製を入れる。
           - 失敗したら `cache.on_error(&AuthError::Sign(e))` に従う。`Retry` なら `continue`、`Stop(m)` なら `stop_message = Some(m)` にして `break` する。
        2. `client.get_session_signed(&token, &sig)` を呼ぶ。
        3. `Ok` なら `complete_auth` を呼んで `return` する。
        4. `Err(e)` なら `cache.on_error(&e)` に従う。
           - `Retry` → 何もしない。
           - `RetryWithFreshSignature` → `state_arc` の `pending_auth.signature = None` にする。
           - `Stop(m)` → `warn!` し、`stop_message = Some(m)` にして `break` する。
      - タイムアウトしたときは、`stop_message = Some("ブラウザでの承認が 5 分以内に確認できませんでした。もう一度ログインしてください".into())` にする。
      - ログには `e.user_message()` だけを出す。token や api_sig は出さない。
      - ループを抜けたら、`lastfm-auth-polling` に `serde_json::json!({ "polling": false, "error": stop_message })` を送る。
      - 旧来の `e.contains("not been authorised") || e.contains("14")` の判定は削除する。
  11. **`commands/auth.rs` の `lastfm_get_session`(手動のフォールバック)。**
      - `pending_auth` から token と署名を取る。
      - ポーリングを止める。
      - 署名が None のときだけ `request_signature` を 1 回呼ぶ。
      - `get_session_signed` を呼ぶ。エラーは `user_message()` で `String` にする。
  12. **`complete_auth` と、`pending_auth_token = None` の箇所。**
      - `complete_auth` のシグネチャは変えない。`session.api_key` は Task 3 で保存する。
      - `pending_auth_token = None` にしている箇所はすべて `pending_auth = None` にする。
  13. **`md5` の削除。** `src-tauri/Cargo.toml` の `md5` の行を削除する。`Cargo.lock` はオフラインのまま cargo が更新する。
- Acceptance:
  - 共通の検証コマンドが成功する。
  - `git grep -n --untracked -e 'LASTFM_API_''SECRET' -e 'md5' -- src-tauri/src src-tauri/Cargo.toml` が 1 件もヒットしない。`lastfm.rs` に `api_secret` フィールドと `fn sign` が無い。
  - `get_session_signed` は署名を取らない。ポーリングのループで署名を取るのは、`cache.get()` が None のときだけである(コードを読んで確かめる)。
  - `lastfm.rs` の `get_token` と `get_session_signed` に、`status.is_client_error()` のような HTTP ステータスによる早期の return が無い。
- Verify: `cd src-tauri && cargo test --lib --offline && cargo clippy --lib --offline -- -D warnings && cd .. && npm run build && ! git grep -n --untracked -e 'LASTFM_API_''SECRET' -e 'md5' -e 'fn sign(' -- src-tauri/src src-tauri/Cargo.toml`

### Task 3: 発行時の api_key を保存し、起動時に不一致なら再ログインを案内する

- 設計書: 8.2(`commands/auth.rs` の項目)、7.4 の 15(既存ユーザーの再ログイン)
- 依存タスク: Task 2
- Files to touch:
  - `src-tauri/src/services/auth_flow.rs`(先頭の `#![allow(dead_code)]` を外すだけ。未使用の項目が残る場合は、`#[cfg(test)]` に移すか削除する)
  - `src-tauri/src/services/lastfm.rs`(`LastfmSession.api_key` に Task 2 で付けた `#[allow(dead_code)]` を外すだけ)
  - `src-tauri/src/models/status.rs`
  - `src-tauri/src/commands/auth.rs`
  - `src-tauri/src/commands/settings.rs`
  - `src-tauri/src/lib.rs`
  - `src/lib/tauriInvoke.ts`
  - `src/components/settings/LastfmSettings.tsx`
  - `src/pages/Dashboard.tsx`
  - `src/i18n/locales/ja.json`
  - `src/i18n/locales/en.json`
- Files NOT to touch: `src-tauri/src/models/settings.rs`(`Settings` にフィールドを足さない)、`src-tauri/src/services/lastfm.rs` の上記以外の部分、`src-tauri/src/services/scrobsign.rs`、`.github/**`
- New dependencies: none
- Steps:
  1. **`AuthStatus` に `notice` を足す。** `models/status.rs` の `AuthStatus` に、次のフィールドを足す。

     ```rust
     #[serde(default, skip_serializing_if = "Option::is_none")]
     pub notice: Option<String>,
     ```

     `AuthStatus { ... }` を作っている箇所すべてに `notice: None` を足す。対象は `lib.rs` 56 行目付近、`commands/auth.rs` の `complete_auth`(46 行目付近)、`lastfm_logout`(259 行目と 296 行目付近)、`commands/settings.rs` の `reset_saved_data`(145 行目と 160 行目付近)。`git grep -n "AuthStatus {" -- src-tauri/src` で漏れが無いことを確かめる。
  2. **Store のキーと補助関数。** `commands/auth.rs` に定数 `pub const STORE_SESSION_API_KEY: &str = "lastfm_session_api_key";` を置く。Store は既存の `STORE_PATH`(`settings.json`)と同じものを使う。補助関数として、次の 3 つを足す。
     - `fn save_session_api_key(app: &AppHandle, api_key: &str)`: `store.set(STORE_SESSION_API_KEY, json!(api_key))` のあと `save()` する。失敗したら `warn!` を出す。
     - `fn load_session_api_key(app: &AppHandle) -> Option<String>`
     - `pub fn clear_session_api_key(app: &AppHandle)`: `store.delete(...)` のあと `save()` する。
  3. **`complete_auth` で保存する。** `store_session_key(&session.key)?` の直後に `save_session_api_key(app, &session.api_key)` を呼ぶ。イベントを送る前に保存を終える。
  4. **ログアウトとリセットで消す。** `lastfm_logout` では、keyring の削除の直後に `clear_session_api_key(&app)` を呼ぶ。`commands/settings.rs` の `reset_saved_data` では、Store を既定値で上書きする前後のどちらかで、`store.delete(crate::commands::auth::STORE_SESSION_API_KEY)` を呼んでから `save()` する。
  5. **`lastfm_get_auth_status` を書き換える。**
     - 引数に `app: AppHandle` を足す。フロントの `invoke` 側の変更は不要。
     - `evaluate_stored_session(load_session_key().is_some(), load_session_api_key(&app).as_deref(), inner.lastfm_client.embedded_api_key())` の結果で、状態を決める。
       - `Valid`: 従来の「session がある」場合と同じ扱い。`authenticated = true` にし、username を補い、`notice = None` にする。
       - `NotLoggedIn`: `authenticated = false`、`username = None`、`notice = None`。
       - `KeyChanged`: `authenticated = false`、`username = None`、`notice = Some("key_changed".into())`。`info!("lastfm session was issued for a different api_key; re-login required")` を出す(値は出さない)。keyring の session key は**消さない**(Design notes を参照)。
     - 既存の `info!` と `warn!` は残してよい。
  6. **フロントの型。** `src/lib/tauriInvoke.ts` の `AuthStatus` に `notice?: string | null;` を足す。
  7. **i18n。** `ja.json` と `en.json` の両方に、同じキーを足す。`src/i18n/types.d.ts` が `ja.json` から型を作るので、キーをそろえる。
     - `lastfm.keyChanged`
       - ja: `"Last.fm のキーが変わったため、再ログインが必要です"`
       - en: `"The Last.fm API key has changed. Please log in again."`
  8. **`LastfmSettings.tsx`。**
     - 未認証の表示の中、ログインのボタンの上に、`lastfmStatus.notice === "key_changed"` のときだけ `<p className="text-xs text-amber-400">{t("lastfm.keyChanged")}</p>` を出す。
     - `lastfm-auth-polling` の購読を `listen<{ polling: boolean; error?: string | null }>` に変え、`polling` が false で `error` があれば `setError(e.payload.error)` を呼ぶ。
  9. **`Dashboard.tsx`。** `!isReady` のときの表示(91 行目と 92 行目付近)で、`lastfm.notice === "key_changed"` なら `dashboard.loginPrompt` の代わりに `t("lastfm.keyChanged")` を出す。
- Acceptance:
  - 共通の検証コマンドが成功する(`npm run build` の `tsc` を含む)。
  - `auth_flow.rs` と `lastfm.rs` に `allow(dead_code)` が無い。
  - `git grep -n "STORE_SESSION_API_KEY" -- src-tauri/src` が、定義、`complete_auth` での保存、ログアウトでの削除、リセットでの削除の 4 種類を含む。
  - `git grep -n "lastfm_session_api_key\|session_api_key" -- src-tauri/src/models/settings.rs src/lib/tauriInvoke.ts` が 0 件(`Settings` に入れていない)。
  - `evaluate_stored_session` のテスト(Task 1)が引き続き通る。
- Verify: `cd src-tauri && cargo test --lib --offline && cargo clippy --lib --offline -- -D warnings && cd .. && npm run build && ! grep -n "allow(dead_code)" src-tauri/src/services/auth_flow.rs src-tauri/src/services/lastfm.rs`

### Task 4: 読み取り系(user.getRecentTracks)のエラーを本文で判定する

- 設計書: 5.11(エラー 10 と 26 を「サービス側の問題」と表示する)。依頼の注意点「HTTP ステータスで弾かず、本文を読む」。
- 依存タスク: Task 1(`parse_lastfm_error` を使う)。Task 2 と Task 3 の後に実行する。
- Files to touch: `src-tauri/src/services/lastfm.rs`
- Files NOT to touch: `src-tauri/src/services/poller.rs`(`"transient:"` で始まるエラーを読み飛ばす既存の約束を維持する)、`src-tauri/src/services/auth_flow.rs`、`src-tauri/src/commands/**`、`src/**`
- New dependencies: none
- Steps:
  1. **判定を純粋な関数にまとめる。** `lastfm.rs` に `pub(crate) fn interpret_read_response(status: u16, body: &str) -> Result<serde_json::Value, String>` を追加する。規則は次のとおり。
     - 本文が JSON として読める場合:
       - `auth_flow::parse_lastfm_error` が `Some(err)` を返したら、`err.code` で分ける。
         - 11、16、29(一時的な障害やレート制限) → `Err(format!("transient: Last.fm error {}: {}", code, message))`
         - 10 と 26 → `Err(format!("Last.fm のサービス側の問題です（error {code}）: {message}"))`
         - その他 → `Err(format!("Last.fm error {code}: {message}"))`
       - エラーが無ければ `Ok(value)` を返す。**ステータスが 4xx でも、本文にエラーが無ければ値として扱う。**
     - JSON として読めない場合:
       - status が 429 か 500 以上 → `Err(format!("transient: Last.fm HTTP {status} error"))`
       - それ以外 → `Err(format!("Last.fm returned non-JSON (HTTP {status}, {} bytes)", body.len()))`
  2. **`get_now_playing` に使う。** 161 行目から 169 行目の「`is_server_error() || is_client_error()` なら早期に return する」処理を削除し、`interpret_read_response(status.as_u16(), &body)?` で JSON を得る。続く `resp["message"]` の確認(178 行目付近)も、この関数に置き換える。
  3. **`get_recent_tracks` に使う。** `send()` の後でステータスを取り、本文を読んで、`interpret_read_response` を使う。既存の `resp["message"]` の確認は削除する。
  4. **送信のエラー。** 両方の関数で、`.send()` のエラーを `e.without_url().to_string()` にする(URL に username が載るので、ログに出さない)。
  5. **テスト。** `lastfm.rs` の `#[cfg(test)] mod tests` に、次のテストを足す。値はダミーにする。
     - 200 の正常な JSON → `Ok`
     - 403 と `{"error":26,"message":"Suspended API key"}` → `Err` で「サービス側の問題」を含み、`transient:` で始まらない
     - 403 と error 10 → 同上
     - 400 と error 6 → `Err("Last.fm error 6: ...")`
     - 500 と error 16 → `transient:` で始まる
     - 502 と HTML の本文 → `transient:` で始まる
     - 404 と HTML の本文 → `non-JSON` を含み、`transient:` で始まらない
- Acceptance:
  - 共通の検証コマンドが成功する。
  - `git grep -n "is_client_error" -- src-tauri/src/services/lastfm.rs` が 0 件。
- Verify: `cd src-tauri && cargo test --lib --offline && cargo clippy --lib --offline -- -D warnings && cd .. && npm run build && ! git grep -n "is_client_error" -- src-tauri/src/services/lastfm.rs`

### Task 5: CI、ビルド設定、README から api_secret を除き、残っていないことを CI で検査する

- 設計書: 8.2(`build.yml`、`README.md`、CI の検査)、9.5
- 依存タスク: Task 2(`src-tauri/src` から secret が消えていること)。Task 3 と Task 4 の後に実行する。
- Files to touch:
  - `scripts/check-no-lastfm-secret.sh`(新規。実行権限を付ける)
  - `.github/workflows/build.yml`
  - `.github/copilot-instructions.md`
  - `src-tauri/build.rs`(コメントだけ)
  - `src-tauri/.env.example`
  - `README.md`
- Files NOT to touch: `src-tauri/src/**`、`src/**`、`scripts/release.mjs`、`DESIGN.md`、`TASKS.md`、`package.json`
- New dependencies: none
- Steps:
  1. **`scripts/check-no-lastfm-secret.sh`(bash)を作る。**
     - `set -euo pipefail` を付け、`cd "$(git rev-parse --show-toplevel)"` でリポジトリのルートへ移る。
     - パターンは `pat='LASTFM_API_''SECRET'` と連結で組み立て、スクリプトの中に連続した文字列として現れないようにする。
     - `git grep -n --untracked -e "$pat" -- src-tauri .github` を実行する。
       - ヒットがあれば、その行と「LASTFM の secret の参照が残っています。署名は scrobsign で行います」を標準エラー出力に出し、`exit 1` する。
       - ヒットが無ければ(`git grep` の終了コードが 1)、`no LASTFM secret references` を出して `exit 0` する。
       - `git grep` が 2 以上で終わったときは、そのまま失敗させる。
     - `chmod +x` する。
  2. **`build.yml`。**
     - `Build and upload release`(116 行目と 117 行目)と `Build only`(133 行目と 134 行目)の `env` から、secret の行を削除する。`LASTFM_API_KEY` と `DISCORD_APP_ID` の行は残す。
     - `jobs:` に独立の job を足す。

       ```yaml
       secret-check:
         runs-on: ubuntu-latest
         steps:
           - uses: actions/checkout@v4
           - name: Check that no Last.fm secret is referenced
             shell: bash
             run: bash scripts/check-no-lastfm-secret.sh
       ```

     - `build` job の `Install Linux dependencies` の直後に、ubuntu だけで動くステップを足す。

       ```yaml
       - name: Run Rust unit tests
         if: matrix.platform == 'ubuntu-22.04'
         working-directory: src-tauri
         run: cargo test --lib
       ```

       このステップには `env` で secret を渡さない。テストは埋め込みの api_key に依存しないこと。
     - 既存のステップの順序とトリガー(`push.tags`、`pull_request`、`workflow_dispatch`)は変えない。
  3. **`src-tauri/build.rs`。** 3 行目のコメントを `// → option_env!("LASTFM_API_KEY") 等で参照可能になる(LASTFM_API_KEY は新しいキーの api_key。秘密ではない。署名は scrobsign が行う)` に変える。コードは変えない。
  4. **`src-tauri/.env.example`。** 内容を次の 3 項目にする。
     - コメント: `# Last.fm の api_key(秘密ではない)。署名は署名サービス scrobsign が行うので、api_secret は不要です`
     - `LASTFM_API_KEY=3c209aa3012b54652e6cbe6be58e1075`
     - `DISCORD_APP_ID=your_application_id_here`(README と合わせるため)
  5. **`README.md`(日本語)。**
     - 「事前準備」の Last.fm の部分を書き換える。Last.fm の API アカウントの作成と Shared Secret の取得の手順を削除し、次の内容を書く。
       - `LASTFM_API_KEY` は scrobsign に登録されたアプリ `scrobcord` の api_key で、秘密ではないこと。値は `src-tauri/.env.example` にあること。
       - この値と違う api_key でビルドすると、ログイン時に「api_key が一致しません」のエラーになること。
     - `.env` の例から、secret の行を削除する。
     - `.env` の置き場所の記述を、`build.rs` の実際の読み込み先に合わせて `src-tauri/.env` に直す(現在の記述は「リポジトリルート」)。
     - 新しい節「Last.fm の署名について」を足す。次の内容を短く書く。
       - Scrobcord は Last.fm の api_secret を持たない。認証(`auth.getToken`、`auth.getSession`)の署名は、署名サービス scrobsign(`https://scrobsign.sukisuki.club`)から受け取る。
       - 署名サービスは、署名に必要なパラメータ(認証の token。scrobble するアプリでは session key(sk)と曲のメタデータ)を受け取るが、保存しない。ログにも値を残さない。
       - 再生履歴などの読み取りは、署名サービスを通さずに Last.fm へ直接問い合わせる。
       - 旧版(v0.0.4 以前)から更新すると、Last.fm のキーが変わったため、1 回の再ログインが必要になる。
     - 秘密の値(secret、sk、token、api_sig)は書かない。
  6. **`.github/copilot-instructions.md`。**
     - 30 行目と 56 行目: `api_secret` の記述を削除し、`session_key` だけを keyring に置くことを書く。
     - 58 行目: 「Last.fm API の署名は、署名サービス scrobsign(`services/scrobsign.rs`)から受け取る。アプリは api_secret を持たない」に変える。
     - イベント仕様の表:
       - `lastfm-status-changed` のペイロードに `notice?: string` を足す。
       - `lastfm-auth-polling` の行(`{ polling: boolean, error?: string }`)を足す。
- Acceptance:
  - 共通の検証コマンドが成功する。
  - `bash scripts/check-no-lastfm-secret.sh` が 0 で終わる。
  - 一時的に `echo 'LASTFM_API_''SECRET' > src-tauri/tmp-check.txt` を作るとスクリプトが 1 で終わることを確かめ、確かめた後でそのファイルを削除する。
  - `git grep -n -e 'LASTFM_API_''SECRET' -- README.md` が 0 件。`git grep -n -E '^[A-Z_]*SECRET[A-Z_]*=' -- README.md src-tauri/.env.example` も 0 件(secret を記入させる行が残っていない)。README や `.env.example` の説明文に「api_secret は不要」などと書くのは構わない。
  - `build.yml` が YAML として正しい(`node -e "require('fs').readFileSync('.github/workflows/build.yml','utf8')"` では不十分なので、`python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/build.yml'))"` で確かめる)。
- Verify: `cd src-tauri && cargo test --lib --offline && cargo clippy --lib --offline -- -D warnings && cd .. && npm run build && bash scripts/check-no-lastfm-secret.sh && python3 -c "import yaml; yaml.safe_load(open('.github/workflows/build.yml'))"`
