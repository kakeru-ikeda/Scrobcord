// Phase 6 で実装
import { Music } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useAppStore } from "../store/appStore";
import { cn } from "../lib/utils";

function StatusDots() {
  const { t } = useTranslation();
  const discord = useAppStore((s) => s.discordStatus);
  const lastfm = useAppStore((s) => s.lastfmStatus);

  return (
    <div className="flex shrink-0 flex-col items-end justify-center gap-1.5">
      <div
        className="flex items-center gap-1.5"
        title={
          lastfm.authenticated
            ? lastfm.username
              ? t("nowPlaying.lastfmTooltipConnected", { username: lastfm.username })
              : t("nowPlaying.lastfmTooltipConnectedNoUser")
            : t("nowPlaying.lastfmTooltipDisconnected")
        }
      >
        <span className="text-xs text-muted-foreground/70">Last.fm</span>
        <span className={`inline-block h-2 w-2 rounded-full ${lastfm.authenticated ? "bg-green-400" : "bg-zinc-600"}`} />
      </div>
      <div
        className="flex items-center gap-1.5"
        title={
          discord.connected
            ? t("nowPlaying.discordTooltipConnected")
            : discord.error
            ? t("nowPlaying.discordTooltipError", { error: discord.error })
            : t("nowPlaying.discordTooltipDisconnected")
        }
      >
        <span className="text-xs text-muted-foreground/70">Discord</span>
        <span className={`inline-block h-2 w-2 rounded-full ${discord.connected ? "bg-green-400" : "bg-zinc-600"}`} />
      </div>
    </div>
  );
}

export default function NowPlayingCard() {
  const { t } = useTranslation();
  const track = useAppStore((s) => s.nowPlaying);

  if (!track) {
    return (
      <div className="flex items-center gap-4 p-4">
        {/* アルバムアートプレースホルダー */}
        <div className="relative h-20 w-20 shrink-0 overflow-hidden rounded-md bg-muted flex items-center justify-center">
          <Music className="h-8 w-8 text-muted-foreground opacity-30" />
        </div>

        {/* テキスト */}
        <div className="flex min-w-0 flex-1 flex-col justify-center gap-1">
          <p className="text-base font-semibold leading-tight text-muted-foreground">{t("nowPlaying.noTrack")}</p>
          <p className="text-sm text-muted-foreground/50">—</p>
        </div>

        <StatusDots />
      </div>
    );
  }

  return (
    <div className="flex items-center gap-4 p-4">
      {/* アルバムアート */}
      <div className="relative h-20 w-20 shrink-0 overflow-hidden rounded-md bg-muted">
        {track.album_art_url ? (
          <img
            src={track.album_art_url}
            alt={track.album}
            className="h-full w-full object-cover"
            draggable={false}
          />
        ) : (
          <div className="flex h-full w-full items-center justify-center">
            <Music className="h-8 w-8 text-muted-foreground opacity-40" />
          </div>
        )}
      </div>

      {/* トラック情報 */}
      <div className="flex min-w-0 flex-1 flex-col justify-center gap-1">
        <p
          className={cn(
            "truncate text-base font-semibold leading-tight text-foreground"
          )}
          title={track.title}
        >
          {track.title}
        </p>
        <p
          className="truncate text-sm text-muted-foreground"
          title={track.artist}
        >
          {track.artist}
        </p>
        {track.album && (
          <p
            className="truncate text-xs text-muted-foreground/70"
            title={track.album}
          >
            {track.album}
          </p>
        )}
      </div>

      {/* 接続ステータス */}
      <StatusDots />
    </div>
  );
}
