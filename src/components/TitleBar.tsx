import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, X, Maximize2 } from "lucide-react";
import { cn } from "../lib/utils";

interface TitleBarProps {
  children?: React.ReactNode;
  className?: string;
}

export default function TitleBar({ children, className }: TitleBarProps) {
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    const win = getCurrentWindow();

    win.isMaximized().then(setIsMaximized);

    let unlisten: (() => void) | null = null;
    win.onResized(() => {
      win.isMaximized().then(setIsMaximized);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, []);

  const handleMinimize = () => getCurrentWindow().minimize();
  const handleMaximize = () => getCurrentWindow().toggleMaximize();
  const handleClose = () => getCurrentWindow().close();

  return (
    <div
      className={cn(
        "flex h-9 shrink-0 select-none items-center",
        className
      )}
      data-tauri-drag-region
      onDoubleClick={handleMaximize}
    >
      {/* 左側コンテンツ（ドラッグ邪魔しないようポインターイベントはボタン等で管理） */}
      <div className="flex flex-1 items-center overflow-hidden px-4" data-tauri-drag-region>
        {children}
      </div>

      {/* ウィンドウ操作ボタン */}
      <div className="flex h-full shrink-0" onDoubleClick={(e) => e.stopPropagation()}>
        <button
          onClick={handleMinimize}
          className="flex h-full w-11 items-center justify-center text-foreground/60 hover:bg-white/10 hover:text-foreground transition-colors"
          aria-label="最小化"
          data-no-drag
        >
          <Minus className="h-3.5 w-3.5" />
        </button>
        <button
          onClick={handleMaximize}
          className="flex h-full w-11 items-center justify-center text-foreground/60 hover:bg-white/10 hover:text-foreground transition-colors"
          aria-label={isMaximized ? "元に戻す" : "最大化"}
          data-no-drag
        >
          {isMaximized ? (
            <Maximize2 className="h-3.5 w-3.5 rotate-180" />
          ) : (
            <Square className="h-3.5 w-3.5" />
          )}
        </button>
        <button
          onClick={handleClose}
          className="flex h-full w-11 items-center justify-center text-foreground/60 hover:bg-red-500 hover:text-white transition-colors"
          aria-label="閉じる"
          data-no-drag
        >
          <X className="h-3.5 w-3.5" />
        </button>
      </div>
    </div>
  );
}
