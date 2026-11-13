import { useCallback, useLayoutEffect, useRef, type RefObject } from "react";
import { getLayoutWidth } from "../utils/layoutWidth";
import css from "./ColumnResizeHandle.module.css";

export function ColumnResizeHandle({
  name,
  target,
  widthRef,
  property,
  minProperty,
  maxProperty,
  defaultProperty,
}: {
  name: string;
  target: string;
  widthRef: RefObject<number>;
  property: string;
  minProperty: string;
  maxProperty: string;
  defaultProperty: string;
}) {
  const bounds = useRef({ min: 0, max: 0 });
  const handle = useRef<HTMLDivElement>(null);
  const frame = useRef<number | null>(null);
  const drag = useRef<{ x: number; width: number; rootSize: number } | null>(
    null
  );
  const clamp = (value: number) =>
    Math.min(
      bounds.current.max,
      Math.max(bounds.current.min, Math.round(value * 16) / 16)
    );
  // Width and accessible values are updated together without rendering the app.
  const apply = useCallback(() => {
    const element = handle.current;
    if (!element) return;
    const value = String(widthRef.current);
    const styleValue = `${value}rem`;
    const column = element.parentElement;
    if (column && column.style.getPropertyValue(property) !== styleValue)
      column.style.setProperty(property, styleValue);
    element.setAttribute("aria-valuemin", String(bounds.current.min));
    element.setAttribute("aria-valuemax", String(bounds.current.max));
    element.setAttribute("aria-valuenow", value);
    element.setAttribute("aria-valuetext", `${value} rem`);
  }, [widthRef, property]);
  const flush = useCallback(() => {
    if (frame.current !== null) cancelAnimationFrame(frame.current);
    frame.current = null;
    apply();
  }, [apply]);
  useLayoutEffect(() => {
    const column = handle.current?.parentElement;
    if (!column) return;
    bounds.current = {
      min: getLayoutWidth(column, minProperty),
      max: getLayoutWidth(column, maxProperty),
    };
    if (!widthRef.current)
      widthRef.current = getLayoutWidth(column, defaultProperty);
    apply();
    return () => {
      if (frame.current !== null) cancelAnimationFrame(frame.current);
    };
  }, [apply, minProperty, maxProperty, defaultProperty, widthRef]);
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
      aria-valuemin={0}
      aria-valuemax={0}
      aria-valuenow={0}
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
                ? bounds.current.min
                : event.key === "End"
                  ? bounds.current.max
                  : undefined;
        if (next === undefined) return;
        event.preventDefault();
        widthRef.current = clamp(next);
        flush();
      }}
    />
  );
}
