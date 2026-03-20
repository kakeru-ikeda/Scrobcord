// Phase 6 で実装
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
              ? `接続済 (${lastfm.username})`
              : "接続済"
            : "未接続"}
        </span>
      </div>
      <div className="flex items-center gap-2">
        <Dot ok={discord.connected} />
        <span className="text-muted-foreground">Discord</span>
        <span className="ml-auto text-xs text-muted-foreground">
          {discord.connected
            ? "接続済"
            : discord.error
            ? `エラー: ${discord.error}`
            : "未接続"}
        </span>
      </div>
    </div>
  );
}
