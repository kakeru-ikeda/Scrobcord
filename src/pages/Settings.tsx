// Phase 7 で実装
interface SettingsProps {
  onBack: () => void;
}

export default function Settings({ onBack }: SettingsProps) {
  return (
    <div className="flex h-screen flex-col bg-background">
      <div
        className="flex h-9 shrink-0 items-center gap-2 px-4"
        data-tauri-drag-region
      >
        <button
          onClick={onBack}
          className="text-xs text-muted-foreground hover:text-foreground"
        >
          ← 戻る
        </button>
        <span className="text-sm font-semibold text-foreground/80">設定</span>
      </div>
      <div className="mx-3 h-px bg-border" />
      <div className="flex-1 p-4 text-muted-foreground text-sm">
        Settings — Phase 7 で実装予定
      </div>
    </div>
  );
}
