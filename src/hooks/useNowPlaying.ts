import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { getNowPlaying } from "../lib/tauriInvoke";
import { useAppStore } from "../store/appStore";
import type { Track } from "../lib/tauriInvoke";

export function useNowPlaying() {
  const setNowPlaying = useAppStore((s) => s.setNowPlaying);

  useEffect(() => {
    getNowPlaying().then(setNowPlaying).catch(console.error);

    const unlisten = listen<{ track: Track | null }>("track-changed", (e) => {
      setNowPlaying(e.payload.track);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [setNowPlaying]);
}
