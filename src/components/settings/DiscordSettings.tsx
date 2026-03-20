// Phase 7 で実装
import { Switch } from "../ui/switch";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import { useAppStore } from "../../store/appStore";
import type { Settings } from "../../lib/tauriInvoke";

// ホワイトリスト置換（Rust 側の format_rpc と同じロジック）
function formatPreview(template: string, title: string, artist: string, album: string): string {
  return template
    .replace("{track}", title)
    .replace("{artist}", artist)
    .replace("{album}", album);
}

interface Props {
  settings: Settings;
  onChange: (patch: Partial<Settings>) => void;
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

export default function DiscordSettings({ settings, onChange }: Props) {
  const track = useAppStore((s) => s.nowPlaying);

  const previewTitle = track?.title ?? "Pretender";
  const previewArtist = track?.artist ?? "Official髭男dism";
  const previewAlbum = track?.album ?? "Editorial";

  return (
    <div className="flex flex-col gap-4">
      {/* Application ID */}
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="app-id">Discord Application ID</Label>
        <Input
          id="app-id"
          value={settings.discord_app_id}
          onChange={(e) => onChange({ discord_app_id: e.target.value })}
          placeholder="例: 1234567890123456789"
          autoComplete="off"
        />
      </div>

      {/* Details フォーマット */}
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="details-fmt">
          Details フォーマット
          <span className="ml-1 text-muted-foreground/60">({`{track} {artist} {album}`} 使用可)</span>
        </Label>
        <Input
          id="details-fmt"
          value={settings.rpc_details_format}
          onChange={(e) => onChange({ rpc_details_format: e.target.value })}
          placeholder="{artist} - {track}"
        />
      </div>

      {/* State フォーマット */}
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="state-fmt">State フォーマット</Label>
        <Input
          id="state-fmt"
          value={settings.rpc_state_format}
          onChange={(e) => onChange({ rpc_state_format: e.target.value })}
          placeholder="{album}"
        />
      </div>

      {/* プレビュー */}
      <div className="rounded-md bg-muted/50 px-3 py-2 text-xs">
        <p className="mb-1 text-muted-foreground">プレビュー（現在のトラック情報）</p>
        <p className="font-medium text-foreground truncate">
          {formatPreview(settings.rpc_details_format, previewTitle, previewArtist, previewAlbum) || "—"}
        </p>
        <p className="text-muted-foreground truncate">
          {formatPreview(settings.rpc_state_format, previewTitle, previewArtist, previewAlbum) || "—"}
        </p>
      </div>

      {/* Toggle 群 */}
      <div className="flex flex-col gap-3">
        <SwitchRow
          label="アルバムアート表示"
          checked={settings.rpc_show_album_art}
          onCheckedChange={(v) => onChange({ rpc_show_album_art: v })}
        />
        <SwitchRow
          label="タイムスタンプ表示"
          checked={settings.rpc_show_timestamp}
          onCheckedChange={(v) => onChange({ rpc_show_timestamp: v })}
        />
        <SwitchRow
          label="Last.fm ボタン表示"
          checked={settings.rpc_show_lastfm_button}
          onCheckedChange={(v) => onChange({ rpc_show_lastfm_button: v })}
        />
        <SwitchRow
          label="Discord RPC 有効"
          checked={settings.discord_enabled}
          onCheckedChange={(v) => onChange({ discord_enabled: v })}
        />
      </div>
    </div>
  );
}
