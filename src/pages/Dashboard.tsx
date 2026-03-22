// Phase 6 で実装
import { useCallback } from "react";
import { Settings, Square, Play } from "lucide-react";
import { useTranslation } from "react-i18next";
import NowPlayingCard from "../components/NowPlayingCard";
import ScrobbleHistory from "../components/ScrobbleHistory";
import TitleBar from "../components/TitleBar";
import UpdateBanner from "../components/UpdateBanner";
import { useAppStore } from "../store/appStore";
import { startPolling, stopPolling } from "../lib/tauriInvoke";
import { useUpdateCheck } from "../hooks/useUpdateCheck";

interface DashboardProps {
  onNavigateSettings: () => void;
}

export default function Dashboard({ onNavigateSettings }: DashboardProps) {
  const { t } = useTranslation();
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
  const { updateInfo, dismiss: dismissUpdate } = useUpdateCheck();

  return (
    <div className="flex h-screen flex-col bg-background">
      {/* タイトルバー */}
      <TitleBar
        actions={
          <>
            <button
              onClick={handleTogglePolling}
              disabled={!isReady}
              title={pollingRunning ? t("dashboard.stopTooltip") : t("dashboard.startTooltip")}
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
              title={t("dashboard.settingsTooltip")}
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

      {/* アップデート通知バナー */}
      {updateInfo?.available && (
        <UpdateBanner updateInfo={updateInfo} onDismiss={dismissUpdate} />
      )}

      {/* メインコンテンツ */}
      <div className="flex flex-1 flex-col overflow-hidden">
        {/* ナウプレイングカード */}
        <div className="shrink-0">
          {!isReady ? (
            <div className="flex flex-col items-center justify-center gap-2 px-6 py-4 text-center text-muted-foreground">
              <p className="text-sm">{t("dashboard.notConnected")}</p>
              <p className="text-xs opacity-70">{t("dashboard.loginPrompt")}</p>
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


      </div>
    </div>
  );
}
