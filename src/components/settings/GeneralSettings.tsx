// Phase 7 で実装
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { RefreshCw, ExternalLink } from "lucide-react";
import { Switch } from "../ui/switch";
import { Slider } from "../ui/slider";
import { Label } from "../ui/label";
import { Button } from "../ui/button";
import { Badge } from "../ui/badge";
import { checkForUpdates, openReleaseUrl } from "../../lib/tauriInvoke";
import type { Settings, UpdateInfo } from "../../lib/tauriInvoke";

interface Props {
  settings: Settings;
  onChange: (patch: Partial<Settings>) => void;
  onResetSavedData: () => void;
}

function SwitchRow({
  label,
  checked,
  onCheckedChange,
  disabled,
}: {
  label: string;
  checked: boolean;
  onCheckedChange: (v: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <div className={`flex items-center justify-between${disabled ? " opacity-40" : ""}`}>
      <Label>{label}</Label>
      <Switch checked={checked} onCheckedChange={onCheckedChange} disabled={disabled} />
    </div>
  );
}

export default function GeneralSettings({ settings, onChange, onResetSavedData }: Props) {
  const { t } = useTranslation();

  const [currentVersion, setCurrentVersion] = useState<string | null>(null);
  const [checking, setChecking] = useState(false);
  const [updateResult, setUpdateResult] = useState<UpdateInfo | null>(null);
  const [checkError, setCheckError] = useState<string | null>(null);

  useEffect(() => {
    getVersion().then(setCurrentVersion).catch(() => setCurrentVersion(null));
  }, []);

  const handleCheckUpdate = async () => {
    setChecking(true);
    setUpdateResult(null);
    setCheckError(null);
    try {
      const info = await checkForUpdates();
      setUpdateResult(info);
    } catch (e) {
      setCheckError(String(e));
    } finally {
      setChecking(false);
    }
  };

  const handleOpenRelease = async (url: string) => {
    try {
      await openReleaseUrl(url);
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <div className="flex flex-col gap-4">
      {/* ポーリング間隔 */}
      <div className="flex flex-col gap-2">
        <div className="flex items-center justify-between">
          <Label>{t("general.pollInterval")}</Label>
          <span className="text-xs font-medium text-foreground">
            {t("general.pollUnit", { seconds: settings.poll_interval_secs })}
          </span>
        </div>
        <Slider
          min={5}
          max={60}
          step={5}
          value={settings.poll_interval_secs}
          onValueChange={(v) => onChange({ poll_interval_secs: v })}
        />
        <div className="flex justify-between text-xs text-muted-foreground/60">
          <span>{t("general.pollMin")}</span>
          <span>{t("general.pollMax")}</span>
        </div>
      </div>

      {/* Toggle 群 */}
      <div className="flex flex-col gap-3">
        <SwitchRow
          label={t("general.startOnLogin")}
          checked={settings.start_on_login}
          onCheckedChange={(v) => onChange({ start_on_login: v })}
        />
        <SwitchRow
          label={t("general.startMinimized")}
          checked={settings.start_minimized}
          onCheckedChange={(v) => onChange({ start_minimized: v })}
          disabled={!settings.start_on_login}
        />
        <SwitchRow
          label={t("general.minimizeToTray")}
          checked={settings.minimize_to_tray}
          onCheckedChange={(v) => onChange({ minimize_to_tray: v })}
        />
        <SwitchRow
          label={t("general.rpcEnabled")}
          checked={settings.rpc_enabled}
          onCheckedChange={(v) => onChange({ rpc_enabled: v })}
        />
      </div>

      {/* 言語選択 */}
      <div className="flex items-center justify-between">
        <Label>{t("general.language")}</Label>
        <select
          value={settings.language}
          onChange={(e) => onChange({ language: e.target.value })}
          className="rounded-md border border-border bg-muted px-2 py-1 text-sm text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
        >
          <option value="ja">日本語</option>
          <option value="en">English</option>
        </select>
      </div>

      {/* バージョン + アップデート確認 */}
      <div className="rounded-md border border-border bg-muted/40 p-3 flex flex-col gap-2">
        <div className="flex items-center justify-between">
          <Label className="text-xs text-muted-foreground">{t("general.version")}</Label>
          <span className="text-xs font-mono text-foreground/80">
            {currentVersion ?? "..."}
          </span>
        </div>

        <Button
          variant="outline"
          size="sm"
          className="w-full"
          disabled={checking}
          onClick={handleCheckUpdate}
        >
          <RefreshCw
            className={`mr-1.5 h-3 w-3 ${checking ? "animate-spin" : ""}`}
          />
          {checking ? t("general.checking") : t("general.checkUpdate")}
        </Button>

        {/* 確認結果 */}
        {updateResult && (
          <div className="flex flex-col gap-1.5">
            {updateResult.available ? (
              <>
                <div className="flex items-center justify-between">
                  <Badge variant="warning">
                    {t("general.updateAvailable", { version: updateResult.latest_version })}
                  </Badge>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  className="w-full text-blue-400 border-blue-500/40 hover:bg-blue-500/10"
                  onClick={() => handleOpenRelease(updateResult.release_url)}
                >
                  <ExternalLink className="mr-1.5 h-3 w-3" />
                  {t("update.openReleasePage")}
                </Button>
              </>
            ) : (
              <Badge variant="success" className="self-center">
                {t("general.upToDate")}
              </Badge>
            )}
          </div>
        )}

        {checkError && (
          <p className="text-xs text-red-400 text-center">{checkError}</p>
        )}
      </div>

      <div className="pt-2">
        <Button
          variant="destructive"
          size="sm"
          className="w-full"
          onClick={onResetSavedData}
        >
          {t("general.resetData")}
        </Button>
      </div>
    </div>
  );
}
