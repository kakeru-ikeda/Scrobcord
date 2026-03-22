// Phase 7 で実装
import { useTranslation } from "react-i18next";
import { Switch } from "../ui/switch";
import { Slider } from "../ui/slider";
import { Label } from "../ui/label";
import { Button } from "../ui/button";
import type { Settings } from "../../lib/tauriInvoke";

interface Props {
  settings: Settings;
  onChange: (patch: Partial<Settings>) => void;
  onResetSavedData: () => void;
}

function SwitchRow({
  label,
  checked,
  onCheckedChange,
}: {
  label: string;
  checked: boolean;
  onCheckedChange: (v: boolean) => void;
}) {
  return (
    <div className="flex items-center justify-between">
      <Label>{label}</Label>
      <Switch checked={checked} onCheckedChange={onCheckedChange} />
    </div>
  );
}

export default function GeneralSettings({ settings, onChange, onResetSavedData }: Props) {
  const { t } = useTranslation();

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
