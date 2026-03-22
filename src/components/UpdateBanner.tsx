import { X, ArrowUpCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { openReleaseUrl } from "../lib/tauriInvoke";
import type { UpdateInfo } from "../lib/tauriInvoke";

interface UpdateBannerProps {
  updateInfo: UpdateInfo;
  onDismiss: () => void;
}

/**
 * アップデートが利用可能な場合に Dashboard 上部に表示するバナー。
 * 「リリースページへ」ボタンでブラウザを開き、✕ で非表示にできる。
 */
export default function UpdateBanner({ updateInfo, onDismiss }: UpdateBannerProps) {
  const { t } = useTranslation();

  const handleOpen = async () => {
    if (updateInfo.release_url) {
      try {
        await openReleaseUrl(updateInfo.release_url);
      } catch (e) {
        console.error("リリースページを開けませんでした:", e);
      }
    }
  };

  return (
    <div className="flex items-center gap-2 bg-blue-600/20 border border-blue-500/40 text-blue-200 text-xs px-3 py-2">
      <ArrowUpCircle size={14} className="shrink-0 text-blue-400" />
      <span className="flex-1 truncate">
        {t("update.available", { version: updateInfo.latest_version })}
      </span>
      <button
        onClick={handleOpen}
        className="shrink-0 rounded px-2 py-0.5 bg-blue-500/30 hover:bg-blue-500/50 transition-colors whitespace-nowrap"
      >
        {t("update.openReleasePage")}
      </button>
      <button
        onClick={onDismiss}
        aria-label={t("update.dismiss")}
        className="shrink-0 text-blue-300/60 hover:text-blue-200 transition-colors"
      >
        <X size={13} />
      </button>
    </div>
  );
}
