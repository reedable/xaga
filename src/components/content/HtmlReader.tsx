import { useCallback, useEffect, useRef, useState } from "react";
import { v4 as uuidv4 } from "uuid";
import { useEvents } from "../../hooks/useEvents";
import {
  FIND_CLEAR,
  FIND_IN_PAGE,
  FIND_NAVIGATE,
  FIND_OPEN,
  FIND_RESULT,
  HIGHLIGHT_CLICKED,
  OPEN_LINK_URL,
  SCROLL_TO_ANNOTATION,
  SELECTION_CLEARED,
  SELECTION_MADE,
} from "../../text/buildEntryHtml";
import type { Annotation } from "../../types";
import cx from "../../utils/cx";
import Logger from "../../utils/Logger";
import {
  AnnotationTooltip,
  type AnnotationTooltipHandle,
} from "./AnnotationTooltip";
import { FindBar } from "./FindBar";
import css from "./HtmlReader.module.css";

export interface HtmlReaderSelectionRect {
  top: number;
  left: number;
  width: number;
  height: number;
}

interface TooltipState {
  annotationId: string;
  color: string | null;
  note?: string | null;
  text: string[];
  occurrence: number[];
  position: { x: number; y: number };
}

interface HtmlReaderProps {
  annotations?: Annotation[];
  className?: string;
  entryId?: string;
  html: string;
  scrollToAnnotationId?: string;
  onClickLink?: (url: string) => void; //outgoing links in the html content
  onSaveAnnotation?: (
    annotation: Omit<Annotation, "createdAt" | "updatedAt">
  ) => void;
  onRemoveAnnotation?: (
    annotation: Pick<Annotation, "id" | "reference">
  ) => void;
}

export function HtmlReader({
  annotations,
  className,
  entryId,
  html,
  scrollToAnnotationId,
  onClickLink,
  onSaveAnnotation,
  onRemoveAnnotation,
}: HtmlReaderProps) {
  const { onFind } = useEvents();
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const tooltipRef = useRef<AnnotationTooltipHandle | null>(null);
  const [tooltip, setTooltip] = useState<TooltipState | null>(null);
  const [findOpen, setFindOpen] = useState(false);
  const [findQuery, setFindQuery] = useState("");
  const [findResult, setFindResult] = useState<{
    total: number;
    current: number;
  } | null>(null);

  /** Convert iframe-local rect to fixed viewport coords, positioned below selection. */
  const toViewportPosition = useCallback((rect: HtmlReaderSelectionRect) => {
    const iframeBounds = iframeRef.current?.getBoundingClientRect();
    if (!iframeBounds) return { x: rect.left, y: rect.top + rect.height + 6 };
    return {
      x: iframeBounds.left + rect.left,
      y: iframeBounds.top + rect.top + rect.height + 6,
    };
  }, []);

  const handleSelectionMade = useCallback(
    (texts: string[], occurrences: number[], rect: HtmlReaderSelectionRect) => {
      // Annotation UI is unavailable until the Entry exists.
      if (!entryId) return;
      setTooltip({
        annotationId: uuidv4(),
        text: texts,
        occurrence: occurrences,
        color: null,
        position: toViewportPosition(rect),
      });
    },
    [entryId, toViewportPosition]
  );

  const handleClickHighlight = useCallback(
    (annotationId: string, _text: string, rect: HtmlReaderSelectionRect) => {
      if (!entryId) return;
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
    [annotations, entryId, toViewportPosition]
  );

  const handleChangeTooltip = useCallback(
    (color: string | null, note: string) => {
      if (!tooltip || !entryId) return;

      if (tooltip.occurrence.length > 0) {
        const id = tooltip.annotationId;
        iframeRef.current?.contentWindow?.postMessage(
          { type: "remove-highlight", annotationId: id },
          "*"
        );
        if (color !== null) {
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
            "*"
          );
        }
      }

      if (color !== null && color !== tooltip.color) {
        onSaveAnnotation?.({
          id: tooltip.annotationId,
          title: tooltip.text.join(" "),
          reference: entryId,
          text: tooltip.text,
          occurrence: tooltip.occurrence,
          color,
          note: note || undefined,
        });
      } else if (color === null && tooltip.color !== null) {
        onRemoveAnnotation?.({ id: tooltip.annotationId, reference: entryId });
      }

      setTooltip((prev) => (prev ? { ...prev, color, note } : null));
    },
    [tooltip, onSaveAnnotation, onRemoveAnnotation, entryId]
  );

  const handleCloseTooltip = useCallback(async () => {
    if (!tooltip || !entryId) {
      // nothing to do (or Entry not ready)
    } else if (tooltip.color === null) {
      onRemoveAnnotation?.({ id: tooltip.annotationId, reference: entryId });
    } else {
      const annotation = {
        id: tooltip.annotationId,
        title: tooltip.text.join(" "),
        reference: entryId,
        text: tooltip.text,
        occurrence: tooltip.occurrence,
        color: tooltip.color,
        note: tooltip.note ?? undefined,
      };
      onSaveAnnotation?.(annotation);
      await tooltipRef.current?.saveAnnotationToTopics(
        annotation as Annotation
      );
    }

    setTooltip(null);
  }, [onRemoveAnnotation, onSaveAnnotation, entryId, tooltip]);

  const handleIframeLoad = useCallback(() => {
    if (annotations?.length) {
      iframeRef.current?.contentWindow?.postMessage(
        { type: "apply-highlights", annotations },
        "*"
      );
    }
    if (scrollToAnnotationId) {
      iframeRef.current?.contentWindow?.postMessage(
        { type: SCROLL_TO_ANNOTATION, annotationId: scrollToAnnotationId },
        "*"
      );
    }
    if (findOpen && findQuery) {
      iframeRef.current?.contentWindow?.postMessage(
        { type: FIND_IN_PAGE, query: findQuery },
        "*"
      );
    }
  }, [annotations, scrollToAnnotationId, findOpen, findQuery]);

  const handleFindClose = useCallback(() => {
    setFindOpen(false);
    setFindQuery("");
    setFindResult(null);
    iframeRef.current?.contentWindow?.postMessage({ type: FIND_CLEAR }, "*");
  }, []);

  const handleFindNavigate = useCallback((direction: 1 | -1) => {
    iframeRef.current?.contentWindow?.postMessage(
      { type: FIND_NAVIGATE, direction },
      "*"
    );
  }, []);

  // Send updated query to iframe whenever it changes while find bar is open.
  useEffect(() => {
    if (!findOpen) return;
    iframeRef.current?.contentWindow?.postMessage(
      { type: FIND_IN_PAGE, query: findQuery },
      "*"
    );
  }, [findQuery, findOpen]);

  // Clear stale result count when switching to a different entry.
  useEffect(() => {
    (async () => setFindResult(null))();
  }, [entryId]);

  // Edit → Find… menu item (Cmd-F).
  useEffect(() => onFind(() => setFindOpen(true)), [onFind]);

  useEffect(() => {
    const blob = new Blob([html], { type: "text/html" });
    const url = URL.createObjectURL(blob);
    if (iframeRef.current) iframeRef.current.src = url;
    return () => URL.revokeObjectURL(url);
  }, [html]);

  useEffect(() => {
    function handleMessage(e: MessageEvent) {
      const log = Logger.getLogger("handleMessage");

      // Only accept messages from our own reader iframe. Without this an
      // unsandboxed sibling frame (e.g. the PDF viewer) or any window holding a
      // reference to this one could spoof OPEN_LINK_URL and drive link loading.
      if (e.source !== iframeRef.current?.contentWindow) return;

      log.log("handleMessage", e.data);

      const msg = e.data;
      if (!msg?.type) return;

      switch (msg.type) {
        case OPEN_LINK_URL:
          setTooltip(null);
          if (msg.url) onClickLink?.(msg.url);
          break;
        case SELECTION_MADE:
          handleSelectionMade(
            msg.texts as string[],
            msg.occurrences as number[],
            msg.rect as HtmlReaderSelectionRect
          );
          break;
        case SELECTION_CLEARED:
          // SELECTION_CLEARED normally means "no text is selected", but when a
          // tooltip is open it must be properly closed: saving if color is set,
          // or calling remove again (harmlessly) if color was already cleared.
          // handleCloseTooltip() handles both; setTooltip(null) would not.
          handleCloseTooltip();
          break;
        case HIGHLIGHT_CLICKED:
          handleClickHighlight(
            msg.annotationId,
            msg.text,
            msg.rect as HtmlReaderSelectionRect
          );
          break;
        case FIND_OPEN:
          setFindOpen(true);
          break;
        case FIND_RESULT:
          setFindResult({
            total: msg.total as number,
            current: msg.current as number,
          });
          break;
        case FIND_CLEAR:
          setFindOpen(false);
          setFindQuery("");
          setFindResult(null);
          break;
      }
    }

    window.addEventListener("message", handleMessage);
    return () => window.removeEventListener("message", handleMessage);
  }, [
    onClickLink,
    handleSelectionMade,
    handleClickHighlight,
    handleCloseTooltip,
  ]);

  return (
    <div className={cx(css.HtmlReader, className)}>
      <iframe
        ref={iframeRef}
        onLoad={handleIframeLoad}
        sandbox="allow-scripts"
        allow="autoplay 'none'"
        title="Entry reader"
      />

      {findOpen && (
        <FindBar
          query={findQuery}
          current={findResult?.current ?? 0}
          total={findResult?.total ?? 0}
          onChange={setFindQuery}
          onNavigate={handleFindNavigate}
          onClose={handleFindClose}
        />
      )}

      {tooltip && (
        <AnnotationTooltip
          ref={tooltipRef}
          annotation={
            annotations?.find((a) => a.id === tooltip.annotationId) ?? null
          }
          position={tooltip.position}
          text={tooltip.text}
          color={tooltip.color}
          note={tooltip.note}
          onChange={handleChangeTooltip}
          onClose={handleCloseTooltip}
        />
      )}
    </div>
  );
}
