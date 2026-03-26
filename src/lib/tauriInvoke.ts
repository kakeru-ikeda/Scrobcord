import { invoke } from "@tauri-apps/api/core";

// ---------------------------------------------------------------------------
// Types (Rust 側の構造体と対応)
// ---------------------------------------------------------------------------

export interface Track {
  title: string;
  artist: string;
  album: string;
  album_art_url: string | null;
  url: string | null;
  timestamp: number | null;
}

export interface AuthStatus {
  authenticated: boolean;
  username?: string;
}

export interface DiscordStatus {
  connected: boolean;
  error?: string;
}

export interface Settings {
  lastfm_username: string;
  discord_app_id: string;
  rpc_enabled: boolean;
  rpc_details_format: string;
  rpc_state_format: string;
  rpc_name_format: string;
  rpc_use_listening_type: boolean;
  rpc_show_album_art: boolean;
  poll_interval_secs: number;
  start_on_login: boolean;
  start_minimized: boolean;
  minimize_to_tray: boolean;
  language: string;
}

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

export const lastfmGetAuthToken = () =>
  invoke<void>("lastfm_get_auth_token");

export const lastfmGetSession = () =>
  invoke<void>("lastfm_get_session");

export const lastfmCancelAuth = () =>
  invoke<void>("lastfm_cancel_auth");

export const lastfmLogout = () =>
  invoke<void>("lastfm_logout");

export const lastfmGetAuthStatus = () =>
  invoke<AuthStatus>("lastfm_get_auth_status");

// ---------------------------------------------------------------------------
// Discord
// ---------------------------------------------------------------------------

export const discordConnect = () =>
  invoke<void>("discord_connect");

export const discordDisconnect = () =>
  invoke<void>("discord_disconnect");

export const discordGetStatus = () =>
  invoke<DiscordStatus>("discord_get_status");

// ---------------------------------------------------------------------------
// Polling
// ---------------------------------------------------------------------------

export const startPolling = () =>
  invoke<void>("start_polling");

export const stopPolling = () =>
  invoke<void>("stop_polling");

export const getNowPlaying = () =>
  invoke<Track | null>("get_now_playing");

export const getPollingStatus = () =>
  invoke<boolean>("get_polling_status");

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

export const getSettings = () =>
  invoke<Settings>("get_settings");

export const saveSettings = (settings: Settings) =>
  invoke<void>("save_settings", { settings });

export const resetSavedData = () =>
  invoke<void>("reset_saved_data");

// ---------------------------------------------------------------------------
// Scrobble 履歴
// ---------------------------------------------------------------------------

export interface ScrobbledTrack {
  title: string;
  artist: string;
  album: string;
  album_art_url: string | null;
  url: string | null;
  /** UNIX 秒（nowplaying 時は null） */
  timestamp: number | null;
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

// ---------------------------------------------------------------------------
// アップデート確認
// ---------------------------------------------------------------------------

export interface UpdateInfo {
  /** 新しいバージョンが利用可能か */
  available: boolean;
  /** GitHub Releases の最新バージョン文字列（例: "0.2.0"） */
  latest_version: string;
  /** 現在インストールされているバージョン */
  current_version: string;
  /** GitHub Releases ページの URL */
  release_url: string;
}

export const checkForUpdates = () =>
  invoke<UpdateInfo>("check_for_updates");

export const openReleaseUrl = (url: string) =>
  invoke<void>("open_release_url", { url });
