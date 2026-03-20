import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useAppStore } from "../store/appStore";
import {
  discordGetStatus,
  getPollingStatus,
  lastfmGetAuthStatus,
  type AuthStatus,
  type DiscordStatus,
} from "../lib/tauriInvoke";

export function useConnectionStatus() {
  const setDiscordStatus = useAppStore((s) => s.setDiscordStatus);
  const setLastfmStatus = useAppStore((s) => s.setLastfmStatus);
  const setPollingRunning = useAppStore((s) => s.setPollingRunning);

  useEffect(() => {
    (async () => {
      try {
        const auth = await lastfmGetAuthStatus();
        setLastfmStatus(auth);
      } catch {
      }

      // discordConnect() はここで呼ばない。
      // ポーラーの update_discord が初回接続を管理する。
      // フロントエンドから自動接続すると React StrictMode の effect 二重実行などで
      // 接続が上書きされ、nonce リセットや活動の逆戻りが発生するため。

      try {
        const discord = await discordGetStatus();
        setDiscordStatus(discord);
      } catch {
      }

      try {
        const running = await getPollingStatus();
        setPollingRunning(running);
      } catch {
      }
    })();

    const unlistenDiscord = listen<DiscordStatus>(
      "discord-status-changed",
      (e) => setDiscordStatus(e.payload)
    );
    const unlistenLastfm = listen<AuthStatus>(
      "lastfm-status-changed",
      (e) => setLastfmStatus(e.payload)
    );
    const unlistenPolling = listen<{ running: boolean }>(
      "polling-status-changed",
      (e) => setPollingRunning(e.payload.running)
    );

    return () => {
      unlistenDiscord.then((fn) => fn());
      unlistenLastfm.then((fn) => fn());
      unlistenPolling.then((fn) => fn());
    };
  }, [setDiscordStatus, setLastfmStatus, setPollingRunning]);
}
