import { create } from "zustand";
import type { Track, AuthStatus, DiscordStatus } from "../lib/tauriInvoke";

interface AppStore {
  nowPlaying: Track | null;
  discordStatus: DiscordStatus;
  lastfmStatus: AuthStatus;
  pollingRunning: boolean;
  language: string;
  setNowPlaying: (track: Track | null) => void;
  setDiscordStatus: (status: DiscordStatus) => void;
  setLastfmStatus: (status: AuthStatus) => void;
  setPollingRunning: (running: boolean) => void;
  setLanguage: (lang: string) => void;
}

export const useAppStore = create<AppStore>((set) => ({
  nowPlaying: null,
  discordStatus: { connected: false },
  lastfmStatus: { authenticated: false },
  pollingRunning: false,
  language: "en",
  setNowPlaying: (track) => set({ nowPlaying: track }),
  setDiscordStatus: (status) => set({ discordStatus: status }),
  setLastfmStatus: (status) => set({ lastfmStatus: status }),
  setPollingRunning: (running) => set({ pollingRunning: running }),
  setLanguage: (lang) => set({ language: lang }),
}));
