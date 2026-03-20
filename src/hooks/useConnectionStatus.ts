import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useAppStore } from "../store/appStore";
import type { AuthStatus, DiscordStatus } from "../lib/tauriInvoke";

export function useConnectionStatus() {
  const setDiscordStatus = useAppStore((s) => s.setDiscordStatus);
  const setLastfmStatus = useAppStore((s) => s.setLastfmStatus);
  const setPollingRunning = useAppStore((s) => s.setPollingRunning);

  useEffect(() => {
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
