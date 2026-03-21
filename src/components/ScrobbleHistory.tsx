import { Music, ChevronLeft, ChevronRight, RotateCcw } from "lucide-react";
import { Button } from "./ui/button";
import { useScrobbleHistory } from "../hooks/useScrobbleHistory";
import { useAppStore } from "../store/appStore";
import { formatRelativeTime } from "../lib/utils";
import { cn } from "../lib/utils";
import type { ScrobbledTrack } from "../lib/tauriInvoke";

function TrackRow({ track }: { track: ScrobbledTrack }) {
  return (
    <div
      className={cn(
        "flex items-center gap-2 px-3 py-1.5",
        track.now_playing && "bg-green-500/5"
      )}
    >
      {/* アルバムアート 32×32 */}
      <div className="h-8 w-8 shrink-0 overflow-hidden rounded bg-muted">
        {track.album_art_url ? (
          <img
            src={track.album_art_url}
            alt={track.album}
            className="h-full w-full object-cover"
            draggable={false}
          />
        ) : (
          <div className="flex h-full w-full items-center justify-center">
            <Music className="h-4 w-4 text-muted-foreground opacity-40" />
          </div>
        )}
      </div>

      {/* 曲名 + アーティスト */}
      <div className="min-w-0 flex-1">
        <p className="truncate text-xs font-medium leading-tight text-foreground" title={track.title}>
          {track.title}
        </p>
        <p className="truncate text-xs text-muted-foreground leading-tight" title={track.artist}>
          {track.artist}
        </p>
      </div>

      {/* 時刻 or nowplaying バッジ */}
      <div className="shrink-0 text-right">
        {track.now_playing ? (
          <span className="inline-flex items-center gap-1 rounded-full bg-green-500/15 px-1.5 py-0.5 text-[10px] font-medium text-green-400">
            <span className="inline-block h-1.5 w-1.5 animate-pulse rounded-full bg-green-400" />
            いま再生中
          </span>
        ) : (
          <span className="text-[10px] text-muted-foreground">
            {track.timestamp ? formatRelativeTime(track.timestamp) : ""}
          </span>
        )}
      </div>
    </div>
  );
}

function SkeletonRow() {
  return (
    <div className="flex items-center gap-2 px-3 py-1.5">
      <div className="h-8 w-8 shrink-0 animate-pulse rounded bg-muted" />
      <div className="flex-1 space-y-1">
        <div className="h-2.5 w-3/4 animate-pulse rounded bg-muted" />
        <div className="h-2 w-1/2 animate-pulse rounded bg-muted" />
      </div>
      <div className="h-2 w-10 animate-pulse rounded bg-muted" />
    </div>
  );
}

export default function ScrobbleHistory() {
  const authenticated = useAppStore((s) => s.lastfmStatus.authenticated);
  const { data, loading, error, page, fetchPage } = useScrobbleHistory();

  if (!authenticated) {
    return null;
  }

  const totalPages = data?.total_pages ?? 0;
  const canPrev = page > 1 && !loading;
  const canNext = page < totalPages && !loading;

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      {/* ヘッダー */}
      <div className="flex shrink-0 items-center justify-between px-3 py-1.5">
        <span className="text-xs font-semibold text-foreground/70">再生履歴</span>
        <div className="flex items-center gap-1.5">
          {data && (
            <span className="text-[10px] text-muted-foreground">
              {page} / {totalPages} ページ
            </span>
          )}
          <button
            onClick={() => fetchPage(page)}
            disabled={loading}
            className="rounded p-0.5 text-muted-foreground transition-colors hover:text-foreground disabled:opacity-40"
            title="再読み込み"
          >
            <RotateCcw className="h-3 w-3" />
          </button>
        </div>
      </div>

      {/* トラックリスト */}
      <div className="min-h-0 flex-1 overflow-y-auto">
        {error ? (
          <div className="flex flex-col items-center gap-2 py-6 text-center">
            <p className="text-xs text-destructive">{error}</p>
            <Button variant="outline" size="sm" onClick={() => fetchPage(page)}>
              再試行
            </Button>
          </div>
        ) : loading && !data ? (
          // 初回ローディング: スケルトン表示
          Array.from({ length: 10 }).map((_, i) => <SkeletonRow key={i} />)
        ) : data && data.tracks.length > 0 ? (
          data.tracks.map((track, i) => (
            <TrackRow key={`${track.timestamp ?? "np"}-${i}`} track={track} />
          ))
        ) : data && data.tracks.length === 0 ? (
          <div className="flex items-center justify-center py-6 text-xs text-muted-foreground">
            履歴がありません
          </div>
        ) : null}
      </div>

      {/* ページネーションバー */}
      {data && totalPages > 1 && (
        <div className="flex shrink-0 items-center justify-center gap-2 border-t border-border py-1.5">
          <Button
            variant="ghost"
            size="sm"
            className="h-6 w-6 p-0"
            disabled={!canPrev}
            onClick={() => fetchPage(page - 1)}
          >
            <ChevronLeft className="h-3.5 w-3.5" />
          </Button>
          <span className="min-w-[5rem] text-center text-xs text-muted-foreground">
            {page} / {totalPages}
          </span>
          <Button
            variant="ghost"
            size="sm"
            className="h-6 w-6 p-0"
            disabled={!canNext}
            onClick={() => fetchPage(page + 1)}
          >
            <ChevronRight className="h-3.5 w-3.5" />
          </Button>
        </div>
      )}
    </div>
  );
}
