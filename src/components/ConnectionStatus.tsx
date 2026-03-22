// Phase 6 で実装
import { useTranslation } from "react-i18next";
import { useAppStore } from "../store/appStore";

function Dot({ ok }: { ok: boolean }) {
  return (
    <span
      className={`inline-block h-2 w-2 rounded-full ${
        ok ? "bg-green-400" : "bg-zinc-600"
      }`}
    />
  );
}

export default function ConnectionStatus() {
  const { t } = useTranslation();
  const discord = useAppStore((s) => s.discordStatus);
  const lastfm = useAppStore((s) => s.lastfmStatus);

  return (
    <div className="flex flex-col gap-2 px-4 py-2 text-sm">
      <div className="flex items-center gap-2">
        <Dot ok={lastfm.authenticated} />
        <span className="text-muted-foreground">Last.fm</span>
        <span className="ml-auto text-xs text-muted-foreground">
          {lastfm.authenticated
            ? lastfm.username
              ? t("connection.connectedWithUser", { username: lastfm.username })
              : t("connection.connected")
            : t("connection.notConnected")}
        </span>
      </div>
      <div className="flex items-center gap-2">
        <Dot ok={discord.connected} />
        <span className="text-muted-foreground">Discord</span>
        <span className="ml-auto text-xs text-muted-foreground">
          {discord.connected
            ? t("connection.connected")
            : discord.error
            ? t("connection.error", { error: discord.error })
            : t("connection.notConnected")}
        </span>
      </div>
    </div>
  );
}
