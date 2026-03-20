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
  const previewArtUrl = track?.album_art_url ?? null;

  return (
    <div className="flex flex-col gap-4">
      {/* Discord カード風プレビュー */}
      <div className="rounded-lg bg-[#1e1f22] border border-white/10 p-3">
        <p className="text-xs text-white/40 mb-2">Discord プレビュー</p>
        <div className="flex gap-3 items-center">
          {/* アルバムアート */}
          {settings.rpc_show_album_art && (
            <div className="relative w-14 h-14 flex-shrink-0">
              {previewArtUrl ? (
                <img src={previewArtUrl} alt="" className="w-full h-full object-cover rounded" />
              ) : (
                <div className="w-full h-full bg-white/10 rounded flex items-center justify-center text-white/30 text-xl">
                  🎵
                </div>
              )}
            </div>
          )}
          {/* テキスト */}
          <div className="flex flex-col justify-center gap-0.5 min-w-0">
            {settings.rpc_use_listening_type && (
              <p className="text-[11px] text-white/50 truncate">
                {formatPreview(settings.rpc_name_format, previewTitle, previewArtist, previewAlbum) || "—"}を再生中
              </p>
            )}
            <p className="text-sm font-semibold text-white truncate">
              {formatPreview(settings.rpc_details_format, previewTitle, previewArtist, previewAlbum) || "—"}
            </p>
            <p className="text-xs text-white/70 truncate">
              {formatPreview(settings.rpc_state_format, previewTitle, previewArtist, previewAlbum) || "—"}
            </p>
            {settings.rpc_show_album_art && (
              <p className="text-[11px] text-white/40 truncate">
                {previewAlbum}
              </p>
            )}
          </div>
        </div>
      </div>

      <p className="text-xs text-muted-foreground">
        使用可能: <code className="bg-muted px-1 rounded">{"{track}"}</code>{" "}
        <code className="bg-muted px-1 rounded">{"{artist}"}</code>{" "}
        <code className="bg-muted px-1 rounded">{"{album}"}</code>
      </p>

      {/* アクティビティ名（Listening type 時） */}
      {settings.rpc_use_listening_type && (
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="name-fmt">
            アクティビティ名
            <span className="ml-1.5 text-xs text-muted-foreground/70">「〇〇 を聴いています」の部分</span>
          </Label>
          <Input
            id="name-fmt"
            value={settings.rpc_name_format}
            onChange={(e) => onChange({ rpc_name_format: e.target.value })}
            placeholder="{track}"
          />
        </div>
      )}

      {/* 1行目 Details */}
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="details-fmt">
          1行目
          <span className="ml-1.5 text-xs text-muted-foreground/70">太字で表示（デフォルト: 曲名）</span>
        </Label>
        <Input
          id="details-fmt"
          value={settings.rpc_details_format}
          onChange={(e) => onChange({ rpc_details_format: e.target.value })}
          placeholder="{track}"
        />
      </div>

      {/* 2行目 State */}
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="state-fmt">
          2行目
          <span className="ml-1.5 text-xs text-muted-foreground/70">細字で表示（デフォルト: アーティスト名）</span>
        </Label>
        <Input
          id="state-fmt"
          value={settings.rpc_state_format}
          onChange={(e) => onChange({ rpc_state_format: e.target.value })}
          placeholder="{artist}"
        />
      </div>

      {/* Toggle 群 */}
      <div className="flex flex-col gap-3">
        <SwitchRow
          label="音楽アクティビティ表示 (Listening)"
          checked={settings.rpc_use_listening_type}
          onCheckedChange={(v) => onChange({ rpc_use_listening_type: v })}
        />
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
