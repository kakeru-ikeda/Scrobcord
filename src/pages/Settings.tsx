// Phase 7 で実装
import { useEffect, useRef, useState } from "react";
import LastfmSettings from "../components/settings/LastfmSettings";
import DiscordSettings from "../components/settings/DiscordSettings";
import GeneralSettings from "../components/settings/GeneralSettings";
import { cn } from "../lib/utils";
import { getSettings, resetSavedData, saveSettings } from "../lib/tauriInvoke";
import type { Settings } from "../lib/tauriInvoke";

type Tab = "lastfm" | "discord" | "general";

const TABS: { id: Tab; label: string }[] = [
  { id: "lastfm", label: "Last.fm" },
  { id: "discord", label: "Discord RPC" },
  { id: "general", label: "一般" },
];

const DEBOUNCE_MS = 600;

interface SettingsProps {
  onBack: () => void;
}

export default function Settings({ onBack }: SettingsProps) {
  const [tab, setTab] = useState<Tab>("lastfm");
  const [settings, setSettings] = useState<Settings | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const debounceTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  // 初回読み込み
  useEffect(() => {
    getSettings()
      .then(setSettings)
      .catch((e) => setSaveError(String(e)));
  }, []);

  const handleChange = (patch: Partial<Settings>) => {
    if (!settings) return;
    const next = { ...settings, ...patch };
    setSettings(next);

    // デバウンス保存
    if (debounceTimer.current) clearTimeout(debounceTimer.current);
    debounceTimer.current = setTimeout(async () => {
      try {
        await saveSettings(next);
        setSaveError(null);
      } catch (e) {
        setSaveError(String(e));
      }
    }, DEBOUNCE_MS);
  };

  const handleResetSavedData = async () => {
    if (!window.confirm("保存済みの設定とLast.fm認証情報をリセットします。よろしいですか？")) {
      return;
    }

    if (debounceTimer.current) {
      clearTimeout(debounceTimer.current);
      debounceTimer.current = null;
    }

    try {
      await resetSavedData();
      const fresh = await getSettings();
      setSettings(fresh);
      setSaveError(null);
    } catch (e) {
      setSaveError(String(e));
    }
  };

  // アンマウント時に残タイマーをフラッシュ
  useEffect(() => {
    return () => {
      if (debounceTimer.current) clearTimeout(debounceTimer.current);
    };
  }, []);

  return (
    <div className="flex h-screen flex-col bg-background">
      {/* タイトルバー */}
      <div
        className="flex h-9 shrink-0 items-center gap-3 px-4"
        data-tauri-drag-region
      >
        <button
          onClick={onBack}
          className="text-xs text-muted-foreground hover:text-foreground transition-colors"
        >
          ← 戻る
        </button>
        <span className="text-sm font-semibold text-foreground/80">設定</span>
        {saveError && (
          <span className="ml-auto text-xs text-red-400 truncate max-w-[160px]" title={saveError}>
            保存失敗
          </span>
        )}
      </div>

      <div className="mx-3 h-px bg-border" />

      {/* タブ */}
      <div className="flex gap-1 px-3 pt-2">
        {TABS.map((t) => (
          <button
            key={t.id}
            onClick={() => setTab(t.id)}
            className={cn(
              "px-3 py-1.5 text-xs font-medium rounded-md transition-colors",
              tab === t.id
                ? "bg-primary/20 text-primary"
                : "text-muted-foreground hover:text-foreground hover:bg-muted"
            )}
          >
            {t.label}
          </button>
        ))}
      </div>

      <div className="mx-3 mt-2 h-px bg-border" />

      {/* コンテンツ */}
      <div className="flex-1 overflow-y-auto px-4 py-4">
        {settings == null ? (
          <p className="text-xs text-muted-foreground">読み込み中...</p>
        ) : tab === "lastfm" ? (
          <LastfmSettings settings={settings} onChange={handleChange} />
        ) : tab === "discord" ? (
          <DiscordSettings settings={settings} onChange={handleChange} />
        ) : (
          <GeneralSettings
            settings={settings}
            onChange={handleChange}
            onResetSavedData={handleResetSavedData}
          />
        )}
      </div>
    </div>
  );
}
