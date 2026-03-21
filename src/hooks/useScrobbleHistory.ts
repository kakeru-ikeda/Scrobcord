import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { getRecentTracks, type RecentTracksPage } from "../lib/tauriInvoke";
import { useAppStore } from "../store/appStore";

const HISTORY_LIMIT = 20;

export function useScrobbleHistory() {
  const authenticated = useAppStore((s) => s.lastfmStatus.authenticated);

  const [page, setPage]       = useState(1);
  const [data, setData]       = useState<RecentTracksPage | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError]     = useState<string | null>(null);

  // ページキャッシュ（セッション内で保持）
  const cache    = useRef(new Map<number, RecentTracksPage>());
  // 進行中リクエストの dedup（React StrictMode の二重 effect も吸収）
  const inFlight = useRef(new Set<number>());
  // ユーザーが「見たい」ページ（非同期レス競合対策）
  const wantedPage = useRef(1);

  /**
   * 内部フェッチ。
   * foreground=true  → loading / error state を更新する（ユーザー操作）
   * foreground=false → バックグラウンドプリフェッチ（ただし wantedPage が一致すれば表示を更新）
   */
  const doFetch = useCallback(async (p: number, foreground: boolean) => {
    if (!authenticated) return;
    if (cache.current.has(p)) {
      if (wantedPage.current === p) {
        setPage(p);
        setData(cache.current.get(p)!);
        setLoading(false);
        setError(null);
      }
      return;
    }
    if (inFlight.current.has(p)) return;

    inFlight.current.add(p);
    if (foreground) {
      setLoading(true);
      setError(null);
    }

    const MAX_RETRIES = 2;
    let lastError: unknown = null;
    for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
      if (attempt > 0) {
        // 一時的なエラー（Last.fm rate limit 等）はしばらく待ってリトライ
        await new Promise<void>((resolve) => setTimeout(resolve, 2000 * attempt));
      }
      try {
        const result = await getRecentTracks(p, HISTORY_LIMIT);
        cache.current.set(p, result);
        if (wantedPage.current === p) {
          setPage(p);
          setData(result);
          setLoading(false);
          setError(null);
        }
        inFlight.current.delete(p);
        return;
      } catch (e) {
        lastError = e;
      }
    }

    inFlight.current.delete(p);
    if (wantedPage.current === p) {
      setError(lastError instanceof Error ? lastError.message : String(lastError));
      setLoading(false);
    }
  }, [authenticated]);

  /**
   * ユーザー操作からのページ切り替え。
   * キャッシュヒット → 即座に表示 + 隣接ページをバックグラウンドプリフェッチ。
   * キャッシュミス  → loading 表示してフェッチ + 完了後に隣接をプリフェッチ。
   */
  const fetchPage = useCallback(async (p: number) => {
    if (!authenticated) return;
    wantedPage.current = p;

    const cached = cache.current.get(p);
    if (cached) {
      setPage(p);
      setData(cached);
      setError(null);
      doFetch(p + 1, false);
      if (p > 1) doFetch(p - 1, false);
      return;
    }

    await doFetch(p, true);

    const fetched = cache.current.get(p);
    if (fetched) {
      if (p + 1 <= fetched.total_pages) doFetch(p + 1, false);
      if (p > 1)                        doFetch(p - 1, false);
    }
  }, [authenticated, doFetch]);

  // 認証状態変化時に page=1 をロード（ログアウト時はキャッシュもクリア）
  useEffect(() => {
    if (authenticated) {
      fetchPage(1);
    } else {
      cache.current.clear();
      inFlight.current.clear();
      wantedPage.current = 1;
      setData(null);
      setPage(1);
      setLoading(false);
      setError(null);
    }
  }, [authenticated]); // eslint-disable-line react-hooks/exhaustive-deps

  // track-changed: page=1 のキャッシュを無効化して再取得
  // nowplaying → null（楽曲停止）の場合は Last.fm が scrobble を処理するまで
  // 少し待機してから再取得する（scrobble 送信と重なる rate limit 対策）
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | null = null;
    const unlisten = listen<{ track: unknown }>("track-changed", (event) => {
      cache.current.delete(1);
      if (wantedPage.current === 1) {
        if (timer) clearTimeout(timer);
        const delay = event.payload.track === null ? 1500 : 0;
        if (delay > 0) {
          timer = setTimeout(() => fetchPage(1), delay);
        } else {
          fetchPage(1);
        }
      }
    });
    return () => {
      if (timer) clearTimeout(timer);
      unlisten.then((fn) => fn());
    };
  }, [fetchPage]);

  return { data, loading, error, page, fetchPage };
}
