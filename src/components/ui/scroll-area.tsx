import { useRef, useState, useCallback, useEffect } from "react";
import { cn } from "../../lib/utils";

interface ScrollAreaProps {
  className?: string;
  children: React.ReactNode;
}

/**
 * ネイティブスクロールバーを非表示にして
 * テーマに合わせた薄いカスタムスクロールバーを表示する。
 * - hover / scroll 時にフェードイン、1.5秒後にフェードアウト
 * - サムをドラッグして直接スクロール可能
 */
export function ScrollArea({ className, children }: ScrollAreaProps) {
  const viewportRef = useRef<HTMLDivElement>(null);
  const [thumbHeight, setThumbHeight] = useState(0);
  const [thumbTop, setThumbTop]       = useState(0);
  const [visible, setVisible]         = useState(false);

  // ref で持つことで再レンダリングなしに状態管理
  const dragging        = useRef(false);
  const dragStartY      = useRef(0);
  const dragStartScroll = useRef(0);
  const hideTimer       = useRef<number | undefined>(undefined);

  /** サムの高さ・位置を再計算 */
  const update = useCallback(() => {
    const el = viewportRef.current;
    if (!el) return;
    const ratio = el.clientHeight / el.scrollHeight;
    if (ratio >= 1) {
      setThumbHeight(0);
      return;
    }
    const th      = Math.max(ratio * el.clientHeight, 20);
    const maxTop  = el.clientHeight - th;
    const scrollRatio = el.scrollTop / (el.scrollHeight - el.clientHeight);
    setThumbHeight(th);
    setThumbTop(scrollRatio * maxTop);
  }, []);

  /** 一定時間後にスクロールバーをフェードアウト */
  const scheduleHide = useCallback(() => {
    if (hideTimer.current !== undefined) clearTimeout(hideTimer.current);
    hideTimer.current = window.setTimeout(() => {
      if (!dragging.current) setVisible(false);
    }, 1500);
  }, []);

  /** スクロールバーをフェードイン＋タイマーリセット */
  const showBar = useCallback(() => {
    setVisible(true);
    scheduleHide();
  }, [scheduleHide]);

  // コンテンツサイズが変わったときに再計算
  useEffect(() => {
    const el = viewportRef.current;
    if (!el) return;
    update();
    const ro = new ResizeObserver(update);
    ro.observe(el);
    return () => ro.disconnect();
  }, [update]);

  const handleScroll = useCallback(() => {
    update();
    showBar();
  }, [update, showBar]);

  /** サムのドラッグ操作 */
  const handleMouseDownThumb = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      dragging.current      = true;
      dragStartY.current    = e.clientY;
      dragStartScroll.current = viewportRef.current?.scrollTop ?? 0;
      if (hideTimer.current !== undefined) clearTimeout(hideTimer.current);

      const onMove = (ev: MouseEvent) => {
        const el = viewportRef.current;
        if (!el) return;
        const delta    = ev.clientY - dragStartY.current;
        const scrollMax = el.scrollHeight - el.clientHeight;
        const trackH   = el.clientHeight - thumbHeight;
        if (trackH <= 0) return;
        el.scrollTop = dragStartScroll.current + (delta / trackH) * scrollMax;
        update();
      };

      const onUp = () => {
        dragging.current = false;
        scheduleHide();
        window.removeEventListener("mousemove", onMove);
        window.removeEventListener("mouseup", onUp);
      };

      window.addEventListener("mousemove", onMove);
      window.addEventListener("mouseup", onUp);
    },
    [thumbHeight, update, scheduleHide],
  );

  return (
    <div
      className={cn("relative overflow-hidden", className)}
      onMouseEnter={showBar}
      onMouseLeave={scheduleHide}
    >
      {/* スクロール可能なビューポート（ネイティブバーは非表示） */}
      <div
        ref={viewportRef}
        onScroll={handleScroll}
        className="hide-scrollbar h-full w-full overflow-y-scroll"
      >
        {children}
      </div>

      {/* カスタムスクロールバー（絶対配置でオーバーレイ） */}
      <div
        className={cn(
          "pointer-events-none absolute inset-y-0 right-0 w-2",
          "transition-opacity duration-300",
          visible && thumbHeight > 0 ? "opacity-100" : "opacity-0",
        )}
      >
        <div
          className={cn(
            "pointer-events-auto absolute right-[3px] w-[3px] rounded-full",
            "bg-muted-foreground/30 transition-[background-color,width] duration-150",
            "hover:right-[2px] hover:w-[5px] hover:bg-muted-foreground/55",
            "cursor-default",
          )}
          style={{ top: thumbTop, height: thumbHeight }}
          onMouseDown={handleMouseDownThumb}
        />
      </div>
    </div>
  );
}
