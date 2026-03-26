import { useEffect, useState } from "react";
import { checkForUpdates, UpdateInfo } from "../lib/tauriInvoke";

interface UseUpdateCheckResult {
  updateInfo: UpdateInfo | null;
  checking: boolean;
  error: string | null;
  dismiss: () => void;
  recheck: () => void;
}

/**
 * アプリ起動時に GitHub Releases API でアップデートを確認する hook。
 * ネットワークエラーは握りつぶして UI に影響しない設計。
 */
export function useUpdateCheck(): UseUpdateCheckResult {
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [dismissed, setDismissed] = useState(false);

  const check = async () => {
    setChecking(true);
    setError(null);
    try {
      const info = await checkForUpdates();
      setUpdateInfo(info);
    } catch (e) {
      // ネットワーク断などは静かに無視（コンソールのみ）
      console.warn("アップデート確認に失敗しました:", e);
      setError(String(e));
    } finally {
      setChecking(false);
    }
  };

  // マウント時に自動チェック
  // React StrictMode での二重実行を防ぐため cancelled フラグを使用する
  useEffect(() => {
    let cancelled = false;

    const runAutoCheck = async () => {
      setChecking(true);
      setError(null);
      try {
        const info = await checkForUpdates();
        if (!cancelled) setUpdateInfo(info);
      } catch (e) {
        console.warn("アップデート確認に失敗しました:", e);
        if (!cancelled) setError(String(e));
      } finally {
        if (!cancelled) setChecking(false);
      }
    };

    runAutoCheck();
    return () => { cancelled = true; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const dismiss = () => setDismissed(true);

  return {
    updateInfo: dismissed ? null : updateInfo,
    checking,
    error,
    dismiss,
    recheck: check,
  };
}
