// Phase 6 で実装
import { useCallback } from "react";
import { Settings, Square, Play } from "lucide-react";
import NowPlayingCard from "../components/NowPlayingCard";
import ConnectionStatus from "../components/ConnectionStatus";
import ScrobbleHistory from "../components/ScrobbleHistory";
import TitleBar from "../components/TitleBar";
import { Button } from "../components/ui/button";
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
      <TitleBar>
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

          {/* ボタン行 */}
          <div className="mx-3 mt-3 h-px bg-border" />
          <div className="flex justify-center gap-4 px-4 py-3">
            <Button
              variant={isReady && pollingRunning ? "destructive" : "default"}
              size="default"
              className="w-32"
              onClick={handleTogglePolling}
              disabled={!isReady}
            >
              {isReady && pollingRunning ? (
                <><Square className="mr-1.5 h-4 w-4" /> 停止</>
              ) : (
                <><Play className="mr-1.5 h-4 w-4" /> 開始</>
              )}
            </Button>
            <Button
              variant="outline"
              size="default"
              className="w-32"
              onClick={onNavigateSettings}
            >
              <Settings className="mr-1.5 h-4 w-4" />
              設定
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
