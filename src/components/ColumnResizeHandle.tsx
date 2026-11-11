import { useRef } from "react";
import css from "./ColumnResizeHandle.module.css";

export function ColumnResizeHandle({
  name,
  target,
  value,
  min,
  max,
  onChange,
}: {
  name: string;
  target: string;
  value: number;
  min: number;
  max: number;
  onChange: (width: number) => void;
}) {
  const drag = useRef<{ x: number; width: number; rootSize: number } | null>(
    null
  );
  const resize = (width: number) =>
    onChange(Math.min(max, Math.max(min, Math.round(width * 16) / 16)));
  return (
    <div
      role="separator"
      tabIndex={0}
      aria-label={name}
      aria-controls={target}
      aria-orientation="vertical"
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={value}
      aria-valuetext={`${value} rem`}
      data-testid={`${target}-resize`}
      className={css.Handle}
      onPointerDown={(event) => {
        if (event.button !== 0) return;
        event.preventDefault();
        event.currentTarget.focus({ preventScroll: true });
        event.currentTarget.setPointerCapture(event.pointerId);
        drag.current = {
          x: event.clientX,
          width: value,
          rootSize: parseFloat(
            getComputedStyle(document.documentElement).fontSize
          ),
        };
      }}
      onPointerMove={(event) => {
        if (drag.current)
          resize(
            drag.current.width +
              (event.clientX - drag.current.x) / drag.current.rootSize
          );
      }}
      onPointerUp={(event) => {
        drag.current = null;
        if (event.currentTarget.hasPointerCapture(event.pointerId))
          event.currentTarget.releasePointerCapture(event.pointerId);
      }}
      onLostPointerCapture={() => {
        drag.current = null;
      }}
      onPointerCancel={() => {
        drag.current = null;
      }}
      onKeyDown={(event) => {
        const next =
          event.key === "ArrowLeft"
            ? value - 0.0625
            : event.key === "ArrowRight"
              ? value + 0.0625
              : event.key === "Home"
                ? min
                : event.key === "End"
                  ? max
                  : undefined;
        if (next === undefined) return;
        event.preventDefault();
        resize(next);
      }}
    />
  );
}
