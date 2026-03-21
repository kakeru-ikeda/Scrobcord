// Phase 6 で実装
import { useCallback } from "react";
import { Settings, Square, Play } from "lucide-react";
import NowPlayingCard from "../components/NowPlayingCard";
import ConnectionStatus from "../components/ConnectionStatus";
import ScrobbleHistory from "../components/ScrobbleHistory";
import TitleBar from "../components/TitleBar";
import { useAppStore } from "../store/appStore";
import { startPolling, stopPolling } from "../lib/tauriInvoke";

interface DashboardProps {
  onNavigateSettings: () => void;
}

export default function Dashboard({ onNavigateSettings }: DashboardProps) {
  const pollingRunning = useAppStore((s) => s.pollingRunning);
  const lastfm = useAppStore((s) => s.lastfmStatus);

  const handleTogglePolling = useCallback(async () => {
    try {
      if (pollingRunning) {
        await stopPolling();
      } else {
        await startPolling();
      }
    } catch (e) {
      console.error(e);
    }
  }, [pollingRunning]);

  const isReady = lastfm.authenticated;

  return (
    <div className="flex h-screen flex-col bg-background">
      {/* タイトルバー */}
      <TitleBar
        actions={
          <>
            <button
              onClick={handleTogglePolling}
              disabled={!isReady}
              title={pollingRunning ? "停止" : "開始"}
              className={`flex h-full w-9 items-center justify-center transition-colors ${
                isReady && pollingRunning
                  ? "text-red-400 hover:bg-red-500/20 hover:text-red-300"
                  : "text-foreground/60 hover:bg-white/10 hover:text-foreground disabled:opacity-30 disabled:cursor-not-allowed"
              }`}
            >
              {isReady && pollingRunning ? (
                <Square className="h-3.5 w-3.5" />
              ) : (
                <Play className="h-3.5 w-3.5" />
              )}
            </button>
            <button
              onClick={onNavigateSettings}
              title="設定"
              className="flex h-full w-9 items-center justify-center text-foreground/60 hover:bg-white/10 hover:text-foreground transition-colors"
            >
              <Settings className="h-3.5 w-3.5" />
            </button>
          </>
        }
      >
        <span className="text-sm font-semibold text-foreground/80">Scrobcord</span>
        {pollingRunning && isReady && (
          <span className="ml-2 flex items-center gap-1 text-xs text-green-400">
            <span className="inline-block h-1.5 w-1.5 animate-pulse rounded-full bg-green-400" />
            Scrobbling
          </span>
        )}
      </TitleBar>

      <div className="mx-3 h-px bg-border" />

      {/* メインコンテンツ */}
      <div className="flex flex-1 flex-col overflow-hidden">
        {/* ナウプレイングカード */}
        <div className="shrink-0">
          {!isReady ? (
            <div className="flex flex-col items-center justify-center gap-2 px-6 py-4 text-center text-muted-foreground">
              <p className="text-sm">Last.fm アカウントが未接続です。</p>
              <p className="text-xs opacity-70">設定から Last.fm にログインしてください。</p>
            </div>
          ) : (
            <NowPlayingCard />
          )}
        </div>

        {/* 再生履歴（flex-1 でスクロール可能領域） */}
        {isReady && (
          <>
            <div className="mx-3 h-px shrink-0 bg-border" />
            <ScrobbleHistory />
          </>
        )}

        {/* 接続ステータス */}
        <div className="shrink-0">
          <div className="mx-3 h-px bg-border" />
          <ConnectionStatus />
        </div>
      </div>
    </div>
  );
}
