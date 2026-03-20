// Phase 7 で実装
import { useState } from "react";
import { ExternalLink, LogOut, CheckCircle } from "lucide-react";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import {
  lastfmGetAuthToken,
  lastfmGetSession,
  lastfmLogout,
} from "../../lib/tauriInvoke";
import { useAppStore } from "../../store/appStore";
import type { Settings } from "../../lib/tauriInvoke";

interface Props {
  settings: Settings;
  onChange: (patch: Partial<Settings>) => void;
}

export default function LastfmSettings({ settings, onChange }: Props) {
  const lastfmStatus = useAppStore((s) => s.lastfmStatus);
  const setLastfmStatus = useAppStore((s) => s.setLastfmStatus);

  const [pending, setPending] = useState<"token" | "session" | "logout" | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [awaitingCallback, setAwaitingCallback] = useState(false);

  const handleLogin = async () => {
    setError(null);
    setPending("token");
    try {
      await lastfmGetAuthToken();
      setAwaitingCallback(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setPending(null);
    }
  };

  const handleGetSession = async () => {
    setError(null);
    setPending("session");
    try {
      await lastfmGetSession(""); // token は Rust 側で保持済み
      setAwaitingCallback(false);
    } catch (e) {
      setError(String(e));
    } finally {
      setPending(null);
    }
  };

  const handleLogout = async () => {
    setError(null);
    setPending("logout");
    try {
      await lastfmLogout();
      setLastfmStatus({ authenticated: false });
      setAwaitingCallback(false);
    } catch (e) {
      setError(String(e));
    } finally {
      setPending(null);
    }
  };

  return (
    <div className="flex flex-col gap-4">
      {/* API Key */}
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="api-key">API Key</Label>
        <Input
          id="api-key"
          value={settings.lastfm_api_key}
          onChange={(e) => onChange({ lastfm_api_key: e.target.value })}
          placeholder="Last.fm API Key"
          autoComplete="off"
        />
      </div>

      {/* API Secret */}
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="api-secret">API Secret</Label>
        <Input
          id="api-secret"
          type="password"
          value={settings.lastfm_api_secret}
          onChange={(e) => onChange({ lastfm_api_secret: e.target.value })}
          placeholder="入力後に keyring へ保存されます"
          autoComplete="new-password"
        />
      </div>

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
          <Button
            variant="default"
            size="sm"
            onClick={handleLogin}
            disabled={!settings.lastfm_api_key || !settings.lastfm_api_secret || pending === "token"}
            className="w-full"
          >
            <ExternalLink className="mr-1.5 h-3.5 w-3.5" />
            Last.fm でログイン
          </Button>
          {awaitingCallback && (
            <Button
              variant="outline"
              size="sm"
              onClick={handleGetSession}
              disabled={pending === "session"}
              className="w-full"
            >
              承認完了（セッション取得）
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
