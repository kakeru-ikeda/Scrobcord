// Phase 7 で実装
import { useState, useEffect } from "react";
import { ExternalLink, LogOut, CheckCircle, Loader2, X } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { Button } from "../ui/button";
import {
  lastfmGetAuthToken,
  lastfmCancelAuth,
  lastfmGetAuthStatus,
  lastfmLogout,
} from "../../lib/tauriInvoke";
import { useAppStore } from "../../store/appStore";
import type { Settings } from "../../lib/tauriInvoke";

interface Props {
  settings: Settings;
  onChange: (patch: Partial<Settings>) => void;
}

export default function LastfmSettings({ settings: _settings, onChange: _onChange }: Props) {
  const lastfmStatus = useAppStore((s) => s.lastfmStatus);
  const setLastfmStatus = useAppStore((s) => s.setLastfmStatus);

  const [pending, setPending] = useState<"token" | "logout" | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isPolling, setIsPolling] = useState(false);

  // lastfm-auth-polling イベントを購読して自動完了を受け取る
  useEffect(() => {
    const unlisten = listen<{ polling: boolean }>("lastfm-auth-polling", (e) => {
      setIsPolling(e.payload.polling);
      if (!e.payload.polling) {
        setPending(null);
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const handleLogin = async () => {
    setError(null);
    setPending("token");
    try {
      await lastfmGetAuthToken();
      // ポーリング開始は lastfm-auth-polling イベントで通知されるので
      // ここでは setPending(null) しない（ポーリング終了まで待機表示を継続）
    } catch (e) {
      setError(String(e));
      setPending(null);
      setIsPolling(false);
    }
  };

  const handleCancelAuth = async () => {
    try {
      await lastfmCancelAuth();
    } catch {
      // ignore
    }
    setIsPolling(false);
    setPending(null);
  };

  const handleLogout = async () => {
    setError(null);
    setPending("logout");
    try {
      await lastfmLogout();
      setLastfmStatus({ authenticated: false });
      setIsPolling(false);
    } catch (e) {
      setError(String(e));
    } finally {
      setPending(null);
    }
  };

  // lastfm-status-changed は useConnectionStatus フックが購読済み。
  // 認証完了後に最新状態を取得して確実にストアへ反映する。
  useEffect(() => {
    if (lastfmStatus.authenticated && isPolling) {
      setIsPolling(false);
      setPending(null);
      lastfmGetAuthStatus().then(setLastfmStatus).catch(() => {});
    }
  }, [lastfmStatus.authenticated, isPolling]);

  return (
    <div className="flex flex-col gap-4">
      {/* 認証状態 */}
      {lastfmStatus.authenticated ? (
        <div className="flex items-center justify-between rounded-md bg-green-500/10 px-3 py-2">
          <div className="flex items-center gap-2 text-sm text-green-400">
            <CheckCircle className="h-4 w-4" />
            <span>{lastfmStatus.username ?? "接続済"}</span>
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={handleLogout}
            disabled={pending === "logout"}
          >
            <LogOut className="mr-1 h-3 w-3" />
            ログアウト
          </Button>
        </div>
      ) : (
        <div className="flex flex-col gap-2">
          {isPolling ? (
            /* ブラウザ承認待ちのポーリング中表示 */
            <div className="flex flex-col gap-2">
              <div className="flex items-center gap-2 rounded-md border border-border px-3 py-2 text-sm text-muted-foreground">
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
                <span>ブラウザで承認するのを待っています...</span>
              </div>
              <Button
                variant="ghost"
                size="sm"
                onClick={handleCancelAuth}
                className="w-full text-muted-foreground"
              >
                <X className="mr-1.5 h-3.5 w-3.5" />
                キャンセル
              </Button>
            </div>
          ) : (
            <Button
              variant="default"
              size="sm"
              onClick={handleLogin}
              disabled={pending === "token"}
              className="w-full"
            >
              <ExternalLink className="mr-1.5 h-3.5 w-3.5" />
              Last.fm でログイン
            </Button>
          )}
        </div>
      )}

      {error && (
        <p className="text-xs text-red-400">{error}</p>
      )}
    </div>
  );
}

