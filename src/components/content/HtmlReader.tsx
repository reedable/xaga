import { useCallback, useEffect, useRef, useState } from "react";
import { v4 as uuidv4 } from "uuid";
import { useAppearance } from "../../hooks/useAppearance";
import { useEvents } from "../../hooks/useEvents";
import { useLibraryReadOnly } from "../../hooks/useLibraryReadOnly";
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
  annotationId?: string;
  onClickLink?: (url: string) => void; //outgoing links in the html content
  onSaveAnnotation?: (
    annotation: Omit<Annotation, "createdAt" | "updatedAt" | "__kind">
  ) => Promise<void>;
  onReplaceAnnotation?: (
    oldId: string,
    annotation: Omit<Annotation, "createdAt" | "updatedAt" | "__kind">
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
  annotationId,
  onClickLink,
  onSaveAnnotation,
  onReplaceAnnotation,
  onRemoveAnnotation,
}: HtmlReaderProps) {
  const { onFind } = useEvents();
  const { themeMode } = useAppearance();
  const isLibraryReadOnly = useLibraryReadOnly();
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
      // Read-only instance: creating annotations is a library write; don't offer it.
      // (Backend refuses with E_READ_ONLY regardless — this is the affordance layer.)
      if (isLibraryReadOnly) return;
      setTooltip({
        annotationId: uuidv4(),
        text: texts,
        occurrence: occurrences,
        color: null,
        position: toViewportPosition(rect),
      });
    },
    [entryId, isLibraryReadOnly, toViewportPosition]
  );

  const handleClickHighlight = useCallback(
    (annotationId: string, _text: string, rect: HtmlReaderSelectionRect) => {
      if (!entryId) return;
      const ann = annotations?.find((a) => a.id === annotationId);
      if (!ann) return;
      setTooltip({
        text: ann.text,
        annotationId,
        note: ann.content || null,
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

      // A color change on an already-saved annotation (tooltip.color non-null) is
      // delete/create, not an update: the annotation is replaced under a new id —
      // color is part of its identity — so a crash-stranded change is visible to the
      // backend's id-set diffs. First save of a fresh selection keeps the minted id.
      const isRecolor =
        color !== null && tooltip.color !== null && color !== tooltip.color;
      const id = isRecolor ? uuidv4() : tooltip.annotationId;

      if (tooltip.occurrence.length > 0) {
        iframeRef.current?.contentWindow?.postMessage(
          { type: "remove-highlight", annotationId: tooltip.annotationId },
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

      if (isRecolor) {
        onReplaceAnnotation?.(tooltip.annotationId, {
          id,
          title: tooltip.text.join(" "),
          reference: entryId,
          text: tooltip.text,
          occurrence: tooltip.occurrence,
          color: color as string,
          content: note,
        });
      } else if (color !== null && color !== tooltip.color) {
        void onSaveAnnotation?.({
          id,
          title: tooltip.text.join(" "),
          reference: entryId,
          text: tooltip.text,
          occurrence: tooltip.occurrence,
          color,
          content: note,
        });
      } else if (color === null && tooltip.color !== null) {
        onRemoveAnnotation?.({ id: tooltip.annotationId, reference: entryId });
      }

      setTooltip((prev) =>
        prev ? { ...prev, annotationId: id, color, note } : null
      );
    },
    [
      tooltip,
      onSaveAnnotation,
      onReplaceAnnotation,
      onRemoveAnnotation,
      entryId,
    ]
  );

  const handleCloseTooltip = useCallback(async () => {
    if (!tooltip || !entryId) {
      // nothing to do (or Entry not ready)
    } else if (tooltip.color === null) {
      onRemoveAnnotation?.({ id: tooltip.annotationId, reference: entryId });
    } else {
      const annotation = {
        __kind: "annotation" as const,
        id: tooltip.annotationId,
        title: tooltip.text.join(" "),
        reference: entryId,
        text: tooltip.text,
        occurrence: tooltip.occurrence,
        color: tooltip.color,
        content: tooltip.note ?? "",
      };
      await onSaveAnnotation?.(annotation);
      await tooltipRef.current?.addAnnotationToTopics(annotation as Annotation);
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
    if (annotationId) {
      iframeRef.current?.contentWindow?.postMessage(
        { type: SCROLL_TO_ANNOTATION, annotationId },
        "*"
      );
    }
    if (findOpen && findQuery) {
      iframeRef.current?.contentWindow?.postMessage(
        { type: FIND_IN_PAGE, query: findQuery },
        "*"
      );
    }
  }, [annotations, annotationId, findOpen, findQuery]);

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

  // Reveal the annotation when it changes within an already-loaded article. The
  // initial load is handled by handleIframeLoad; a pre-load run here is a
  // harmless no-op (the message is dropped before the iframe doc is ready).
  useEffect(() => {
    if (!annotationId) return;
    iframeRef.current?.contentWindow?.postMessage(
      { type: SCROLL_TO_ANNOTATION, annotationId },
      "*"
    );
  }, [annotationId]);

  // Clear stale result count when switching to a different entry.
  useEffect(() => {
    (async () => setFindResult(null))();
  }, [entryId]);

  // Edit → Find… menu item (Cmd-F).
  useEffect(() => onFind(() => setFindOpen(true)), [onFind]);

  // Navigate via location.replace, never by setting src: loading an iframe
  // through src leaves it an about:blank → document history pair that WebKit
  // counts in the webview's joint session history, so Back first un-navigates
  // the iframe (blanking the content) before it moves the router. replace()
  // creates no entry and is one of the few cross-origin-permitted operations,
  // so it works on the sandboxed (opaque-origin) frame.
  useEffect(() => {
    const blob = new Blob([html], { type: "text/html" });
    const url = URL.createObjectURL(blob);
    iframeRef.current?.contentWindow?.location.replace(url);
    return () => URL.revokeObjectURL(url);
  }, [html]);

  useEffect(() => {
    function handleMessage(e: MessageEvent) {
      const log = Logger.getLogger("handleMessage");

      // Only accept messages from our own reader iframe. See
      // docs/architecture/security/handling_persisted_content.md, "Rendering,"
      // for the spoofing this guards against.
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
        style={{ colorScheme: themeMode }}
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
