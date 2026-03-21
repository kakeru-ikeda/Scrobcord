// Phase 6 で実装
import { Music } from "lucide-react";
import { useAppStore } from "../store/appStore";
import { cn } from "../lib/utils";

function StatusDots() {
  const discord = useAppStore((s) => s.discordStatus);
  const lastfm = useAppStore((s) => s.lastfmStatus);

  return (
    <div className="flex shrink-0 flex-col items-end justify-center gap-1.5">
      <div className="flex items-center gap-1.5" title={lastfm.authenticated ? `Last.fm: ${lastfm.username ?? "接続済"}` : "Last.fm: 未接続"}>
        <span className="text-xs text-muted-foreground/70">Last.fm</span>
        <span className={`inline-block h-2 w-2 rounded-full ${lastfm.authenticated ? "bg-green-400" : "bg-zinc-600"}`} />
      </div>
      <div className="flex items-center gap-1.5" title={discord.connected ? "Discord: 接続済" : discord.error ? `Discord: ${discord.error}` : "Discord: 未接続"}>
        <span className="text-xs text-muted-foreground/70">Discord</span>
        <span className={`inline-block h-2 w-2 rounded-full ${discord.connected ? "bg-green-400" : "bg-zinc-600"}`} />
      </div>
    </div>
  );
}

export default function NowPlayingCard() {
  const track = useAppStore((s) => s.nowPlaying);

  if (!track) {
    return (
      <div className="flex items-center gap-4 px-4 py-6">
        <div className="flex flex-1 flex-col items-center justify-center gap-2 text-muted-foreground">
          <Music className="h-10 w-10 opacity-40" />
          <p className="text-sm">再生中の楽曲なし</p>
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
