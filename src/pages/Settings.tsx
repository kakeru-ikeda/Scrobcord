// Phase 7 で実装
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import LastfmSettings from "../components/settings/LastfmSettings";
import DiscordSettings from "../components/settings/DiscordSettings";
import GeneralSettings from "../components/settings/GeneralSettings";
import TitleBar from "../components/TitleBar";
import { cn } from "../lib/utils";
import { getSettings, resetSavedData, saveSettings } from "../lib/tauriInvoke";
import type { Settings } from "../lib/tauriInvoke";
import { useAppStore } from "../store/appStore";

type Tab = "lastfm" | "discord" | "general";

const DEBOUNCE_MS = 600;

interface SettingsProps {
  onBack: () => void;
}

export default function Settings({ onBack }: SettingsProps) {
  const { t } = useTranslation();
  const setLanguage = useAppStore((s) => s.setLanguage);

  const TABS: { id: Tab; label: string }[] = [
    { id: "lastfm", label: t("settings.tabs.lastfm") },
    { id: "discord", label: t("settings.tabs.discord") },
    { id: "general", label: t("settings.tabs.general") },
  ];

  const [tab, setTab] = useState<Tab>("lastfm");
  const [settings, setSettings] = useState<Settings | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const debounceTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  // アンマウント時フラッシュ用に最新の settings を ref で保持
  const latestSettingsRef = useRef<Settings | null>(null);

  // 初回読み込み
  useEffect(() => {
    getSettings()
      .then((s) => {
        setSettings(s);
        // 保存済み言語を store に反映
        if (s.language) setLanguage(s.language);
      })
      .catch((e) => setSaveError(String(e)));
  }, []);

  const handleChange = (patch: Partial<Settings>) => {
    if (!settings) return;
    const next = { ...settings, ...patch };
    setSettings(next);
    latestSettingsRef.current = next;

    // 言語変更時は即時反映
    if (patch.language) {
      setLanguage(patch.language);
    }

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
    if (!window.confirm(t("settings.resetConfirm"))) {
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

  // アンマウント時に残タイマーをフラッシュ（保存漏れ防止）
  useEffect(() => {
    return () => {
      if (debounceTimer.current) {
        clearTimeout(debounceTimer.current);
        debounceTimer.current = null;
        if (latestSettingsRef.current) {
          saveSettings(latestSettingsRef.current).catch(console.error);
        }
      }
    };
  }, []);

  return (
    <div className="flex h-screen flex-col bg-background">
      {/* タイトルバー */}
      <TitleBar>
        <button
          onClick={onBack}
          className="text-xs text-muted-foreground hover:text-foreground transition-colors"
        >
          {t("settings.back")}
        </button>
        <span className="ml-2 text-sm font-semibold text-foreground/80">{t("settings.title")}</span>
        {saveError && (
          <span className="ml-auto text-xs text-red-400 truncate max-w-[160px]" title={saveError}>
            {t("settings.saveFailed")}
          </span>
        )}
      </TitleBar>

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
          <p className="text-xs text-muted-foreground">{t("settings.loading")}</p>
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
