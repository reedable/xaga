import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback } from "react";
import css from "./TitleBar.module.css";

interface TitleBarProps {
  className?: string;
}

export function TitleBar({ className }: TitleBarProps) {
  const handleMouseDown = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    const target = e.target as HTMLElement;
    if (target.closest("button, input, a, textarea")) return;
    void getCurrentWindow().startDragging();
  }, []);

  const handleDoubleClick = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      const target = e.target as HTMLElement;
      if (target.closest("button, input, a, textarea")) return;
      void getCurrentWindow().toggleMaximize();
    },
    []
  );

  const classes = [css.TitleBar, className].filter(Boolean).join(" ");

  return (
    <div
      className={classes}
      onMouseDown={handleMouseDown}
      onDoubleClick={handleDoubleClick}
    >
      <div className={css.TrafficLightSpacer} />
    </div>
  );
}
