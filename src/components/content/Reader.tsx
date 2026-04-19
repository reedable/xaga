import { useCallback, useEffect, useRef, useState } from "react";
import { v4 as uuidv4 } from "uuid";
import {
  HIGHLIGHT_CLICKED,
  OPEN_URL,
  SELECTION_CLEARED,
  SELECTION_MADE,
} from "../../text/buildEntryHtml";
import type { Annotation } from "../../types";
import cx from "../../utils/cx";
import { AnnotationTooltip } from "./AnnotationTooltip";
import Logger from "../../utils/Logger";

export interface SelectionRect {
  top: number;
  left: number;
  width: number;
  height: number;
}

interface TooltipState {
  annotationId: string;
  color: string;
  note?: string | null;
  text: string[];
  occurrence: number[];
  position: { x: number; y: number };
}

interface ReaderProps {
  className?: string;
  html: string;
  annotations?: Annotation[];
  onClickUrl?: (url: string) => void;
  onSaveAnnotation?: (annotation: Omit<Annotation, "createdAt">) => void;
  onRemoveAnnotation?: (annotationId: string) => void;
}

export function Reader({
  className,
  html,
  annotations,
  onClickUrl,
  onSaveAnnotation,
  onRemoveAnnotation,
}: ReaderProps) {
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const [tooltip, setTooltip] = useState<TooltipState | null>(null);

  /** Convert iframe-local rect to fixed viewport coords, positioned below selection. */
  const toViewportPosition = useCallback((rect: SelectionRect) => {
    const iframeBounds = iframeRef.current?.getBoundingClientRect();
    if (!iframeBounds) return { x: rect.left, y: rect.top + rect.height + 6 };
    return {
      x: iframeBounds.left + rect.left,
      y: iframeBounds.top + rect.top + rect.height + 6,
    };
  }, []);

  const handleSelectionMade = useCallback(
    (texts: string[], occurrences: number[], rect: SelectionRect) => {
      setTooltip({
        annotationId: uuidv4(),
        text: texts,
        occurrence: occurrences,
        color: "none",
        position: toViewportPosition(rect),
      });
    },
    [toViewportPosition],
  );

  const handleClickHighlight = useCallback(
    (annotationId: string, _text: string, rect: SelectionRect) => {
      const ann = annotations?.find((a) => a.id === annotationId);
      if (!ann) return;
      setTooltip({
        text: ann.text,
        annotationId,
        note: ann.note ?? null,
        color: ann.color,
        occurrence: ann.occurrence,
        position: toViewportPosition(rect),
      });
    },
    [annotations, toViewportPosition],
  );

  const handleChangeTooltip = useCallback(
    (color: string, note: string) => {
      if (tooltip && tooltip.occurrence.length > 0) {
        const id = tooltip.annotationId;
        iframeRef.current?.contentWindow?.postMessage(
          { type: "remove-highlight", annotationId: id },
          "*",
        );
        if (color !== "none") {
          iframeRef.current?.contentWindow?.postMessage(
            {
              type: "apply-highlights",
              annotations: [
                {
                  id,
                  text: tooltip.text,
                  occurrence: tooltip.occurrence,
                  color,
                  note: note || null,
                },
              ],
            },
            "*",
          );
        }
      }
      setTooltip((prev) => (prev ? { ...prev, color, note } : null));
    },
    [tooltip],
  );

  const handleCloseTooltip = useCallback(() => {
    if (tooltip?.color === "none") {
      onRemoveAnnotation?.(tooltip.annotationId);
    } else if (tooltip) {
      onSaveAnnotation?.({
        id: tooltip.annotationId,
        text: tooltip.text,
        occurrence: tooltip.occurrence,
        color: tooltip.color,
        note: tooltip.note ?? undefined,
      });
    }

    setTooltip(null);
  }, [onRemoveAnnotation, onSaveAnnotation, tooltip]);

  const handleIframeLoad = useCallback(() => {
    if (!annotations?.length) return;
    iframeRef.current?.contentWindow?.postMessage(
      { type: "apply-highlights", annotations },
      "*",
    );
  }, [annotations]);

  useEffect(() => {
    const blob = new Blob([html], { type: "text/html" });
    const url = URL.createObjectURL(blob);
    if (iframeRef.current) iframeRef.current.src = url;
    return () => URL.revokeObjectURL(url);
  }, [html]);

  useEffect(() => {
    function handleMessage(e: MessageEvent) {
      const log = Logger.getLogger("handleMessage");
      log.log("handleMessage", e.data);

      const msg = e.data;
      if (!msg?.type) return;

      switch (msg.type) {
        case OPEN_URL:
          if (msg.url) onClickUrl?.(msg.url);
          break;
        case SELECTION_MADE:
          handleSelectionMade(
            msg.texts as string[],
            msg.occurrences as number[],
            msg.rect as SelectionRect,
          );
          break;
        case SELECTION_CLEARED:
          setTooltip(null);
          break;
        case HIGHLIGHT_CLICKED:
          handleClickHighlight(
            msg.annotationId,
            msg.text,
            msg.rect as SelectionRect,
          );
          break;
      }
    }

    window.addEventListener("message", handleMessage);
    return () => window.removeEventListener("message", handleMessage);
  }, [onClickUrl, handleSelectionMade, handleClickHighlight]);

  return (
    <>
      <iframe
        ref={iframeRef}
        className={cx(className)}
        onLoad={handleIframeLoad}
        sandbox="allow-scripts"
        style={{
          width: "100%",
          height: "100%",
          border: "none",
          display: "block",
        }}
        title="Entry reader"
      />

      {tooltip && (
        <AnnotationTooltip
          position={tooltip.position}
          text={tooltip.text}
          color={tooltip.color}
          note={tooltip.note}
          onChange={handleChangeTooltip}
          onClose={handleCloseTooltip}
        />
      )}
    </>
  );
}
