// Phase 6 で実装
import { Music } from "lucide-react";
import { useAppStore } from "../store/appStore";
import { cn } from "../lib/utils";

export default function NowPlayingCard() {
  const track = useAppStore((s) => s.nowPlaying);

  if (!track) {
    return (
      <div className="flex flex-col items-center justify-center gap-3 py-8 text-muted-foreground">
        <Music className="h-12 w-12 opacity-40" />
        <p className="text-sm">再生中の楽曲なし</p>
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
    </div>
  );
}
