import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useAppStore } from "../store/appStore";
import {
  discordConnect,
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

      try {
        await discordConnect();
      } catch {
      }

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
