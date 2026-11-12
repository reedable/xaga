import { useCallback, useLayoutEffect, useRef, type RefObject } from "react";
import css from "./ColumnResizeHandle.module.css";

export function ColumnResizeHandle({
  name,
  target,
  widthRef,
  property,
  min,
  max,
}: {
  name: string;
  target: string;
  widthRef: RefObject<number>;
  property: string;
  min: number;
  max: number;
}) {
  const handle = useRef<HTMLDivElement>(null);
  const frame = useRef<number | null>(null);
  const drag = useRef<{ x: number; width: number; rootSize: number } | null>(
    null
  );
  const clamp = (value: number) =>
    Math.min(max, Math.max(min, Math.round(value * 16) / 16));
  // Width and accessible values are updated together without rendering the app.
  const apply = useCallback(() => {
    const element = handle.current;
    if (!element) return;
    const value = String(widthRef.current);
    const styleValue = `${value}rem`;
    const column = element.parentElement;
    if (column && column.style.getPropertyValue(property) !== styleValue)
      column.style.setProperty(property, styleValue);
    element.setAttribute("aria-valuenow", value);
    element.setAttribute("aria-valuetext", `${value} rem`);
  }, [widthRef, property]);
  const flush = useCallback(() => {
    if (frame.current !== null) cancelAnimationFrame(frame.current);
    frame.current = null;
    apply();
  }, [apply]);
  useLayoutEffect(() => {
    apply();
    return () => {
      if (frame.current !== null) cancelAnimationFrame(frame.current);
    };
  }, [apply]);
  const finish = () => {
    if (!drag.current) return;
    drag.current = null;
    flush();
  };
  return (
    <div
      ref={handle}
      role="separator"
      tabIndex={0}
      aria-label={name}
      aria-controls={target}
      aria-orientation="vertical"
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={min}
      aria-valuetext={`${min} rem`}
      data-testid={`${target}-resize`}
      className={css.Handle}
      onPointerDown={(event) => {
        if (event.button !== 0) return;
        event.preventDefault();
        event.currentTarget.focus({ preventScroll: true });
        event.currentTarget.setPointerCapture(event.pointerId);
        drag.current = {
          x: event.clientX,
          width: widthRef.current,
          rootSize: parseFloat(
            getComputedStyle(document.documentElement).fontSize
          ),
        };
      }}
      onPointerMove={(event) => {
        if (!drag.current) return;
        const next = clamp(
          drag.current.width +
            (event.clientX - drag.current.x) / drag.current.rootSize
        );
        if (next === widthRef.current) return;
        widthRef.current = next;
        if (frame.current === null)
          frame.current = requestAnimationFrame(() => {
            frame.current = null;
            apply();
          });
      }}
      onPointerUp={(event) => {
        finish();
        if (event.currentTarget.hasPointerCapture(event.pointerId))
          event.currentTarget.releasePointerCapture(event.pointerId);
      }}
      onLostPointerCapture={finish}
      onPointerCancel={finish}
      onKeyDown={(event) => {
        const next =
          event.key === "ArrowLeft"
            ? widthRef.current - 0.0625
            : event.key === "ArrowRight"
              ? widthRef.current + 0.0625
              : event.key === "Home"
                ? min
                : event.key === "End"
                  ? max
                  : undefined;
        if (next === undefined) return;
        event.preventDefault();
        widthRef.current = clamp(next);
        flush();
      }}
    />
  );
}
