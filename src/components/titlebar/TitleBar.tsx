import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback } from "react";
import { isCaptureMode } from "../../platform/captureMode";
import css from "./TitleBar.module.css";

interface TitleBarProps {
  className?: string;
}

export function TitleBar({ className }: TitleBarProps) {
  const handleMouseDown = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    if (isCaptureMode || e.button !== 0) return;
    const target = e.target as HTMLElement;
    if (target.closest("button, input, a, textarea")) return;
    void getCurrentWindow().startDragging();
  }, []);

  const handleDoubleClick = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      if (isCaptureMode) return;
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
      <div
        className={css.TrafficLightSpacer}
        data-testid="traffic-light-spacer"
      />
    </div>
  );
}
