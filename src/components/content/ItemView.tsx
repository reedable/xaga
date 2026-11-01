import { Close } from "@mui/icons-material";
import {
  Drawer,
  IconButton,
  Tooltip,
  Typography,
  useMediaQuery,
} from "@mui/material";
import { useCallback, useEffect, useRef, useState, type RefObject } from "react";
import useAbortController from "../../hooks/useAbortController";
import { useContentLoader } from "../../hooks/useContentLoader";
import useRouter from "../../hooks/useRouter";
import cx from "../../utils/cx";
import Logger from "../../utils/Logger";
import ArticleReader, { type ArticleReaderHandle } from "./ArticleReader";
import css from "./ItemView.module.css";
import { PdfReader } from "./PdfReader";
import {
  Inspector,
  type InspectorView,
  type InspectorTarget,
} from "./Inspector";
import { useEvents } from "../../hooks/useEvents";
import type { Annotation } from "../../types";
import { Toolbar } from "./Toolbar";

type ItemViewProps = {
  className?: string;
  searchPanelRef?: RefObject<HTMLDivElement | null>;
};

const logger = Logger.getLogger("ItemView");

export function ItemView({ className, searchPanelRef }: ItemViewProps) {
  const { state, search } = useRouter();
  const searchOpen = search !== undefined;
  const { entryId, annotationId, url, mimeType } = state;

  const currentRouteTarget = useRef({ entryId, annotationId });
  currentRouteTarget.current = { entryId, annotationId };

  const containerRef = useRef<HTMLDivElement>(null);
  const inspectorOpener = useRef<HTMLButtonElement>(null);
  const restoreInspectorFocus = useRef(false);
  const inspectorReturnFocus = useRef<HTMLElement | null>(null);
  const inspectorPanel = useRef<HTMLElement>(null);
  const [narrow, setNarrow] = useState(false);
  const [inspectorOpen, setInspectorOpen] = useState(false);
  const animate = useMediaQuery("(prefers-reduced-motion: no-preference)");
  const [inspectorView, setInspectorView] = useState<InspectorView>("related");
  const [annotationInspectorView, setAnnotationInspectorView] =
    useState<InspectorView>("related");
  const [drawerTarget, setDrawerTarget] = useState<InspectorTarget | null>(
    entryId ? { entryId, annotationId } : null
  );
  const [routeTarget, setRouteTarget] = useState({ entryId, annotationId });
  if (
    routeTarget.entryId !== entryId ||
    routeTarget.annotationId !== annotationId
  ) {
    setRouteTarget({ entryId, annotationId });
    setDrawerTarget(entryId ? { entryId, annotationId } : null);
  }
  const { onAnnotationReplaced, onAnnotationRemoved } = useEvents();
  useEffect(
    () =>
      onAnnotationReplaced(({ oldId, item }) => {
        setDrawerTarget((current) =>
          current?.annotationId === oldId
            ? { ...current, annotationId: item.id }
            : current
        );
      }),
    [onAnnotationReplaced]
  );
  useEffect(
    () =>
      onAnnotationRemoved((id) => {
        setDrawerTarget((current) =>
          current?.annotationId === id ? { entryId: current.entryId } : current
        );
      }),
    [onAnnotationRemoved]
  );

  useEffect(() => {
    if (inspectorOpen || !restoreInspectorFocus.current) return;
    restoreInspectorFocus.current = false;
    // Restore the tooltip opener when it survives; otherwise use the reader.
    if (inspectorReturnFocus.current?.isConnected)
      inspectorReturnFocus.current.focus();
    else containerRef.current?.querySelector("iframe")?.focus();
  }, [inspectorOpen]);
  const closeInspector = () => {
    if (drawerTarget?.annotationId)
      setDrawerTarget({ entryId: drawerTarget.entryId });
    restoreInspectorFocus.current = true;
    setInspectorOpen(false);
  };
  const openAnnotationInspector = useCallback(
    (
      annotation: Pick<Annotation, "id" | "reference">,
      opener?: HTMLElement
    ) => {
      if (
        annotation.reference !== entryId ||
        currentRouteTarget.current.entryId !== entryId ||
        currentRouteTarget.current.annotationId !== annotationId
      )
        return;
      inspectorReturnFocus.current =
        opener ?? containerRef.current?.querySelector("iframe") ?? null;
      setDrawerTarget({
        entryId: annotation.reference,
        annotationId: annotation.id,
      });
      setInspectorOpen(true);
    },
    [entryId, annotationId]
  );
  useEffect(() => {
    const element = containerRef.current;
    if (!element) return;
    const observer = new ResizeObserver(([entry]) => {
      // Include Search’s current width so its animation does not flip
      // the responsive breakpoint halfway through the transition.
      const searchWidth = searchPanelRef?.current?.getBoundingClientRect().width ?? 0;
      setNarrow(entry.contentRect.width + searchWidth < 700);
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, [searchPanelRef]);
  const inspectorHidden = narrow || searchOpen;
  const useInspectorDrawer = inspectorHidden || !!drawerTarget?.annotationId;
  useEffect(() => {
    if (!useInspectorDrawer && inspectorOpen) {
      setInspectorOpen(false);
      requestAnimationFrame(() =>
        inspectorPanel.current
          ?.querySelector<HTMLElement>('[aria-selected="true"]')
          ?.focus()
      );
    }
  }, [useInspectorDrawer, inspectorOpen]);

  const [abort, abortControllerRef] = useAbortController();
  // Resolves a bare URL (link / URL bar) into a presentation and writes the
  // result back onto the route. Returns the inline-PDF blob URL, and the URL
  // (if any) whose downloaded content was rejected.
  const { pdfBlobUrl, blockedUrl, resolve } =
    useContentLoader(abortControllerRef);

  // Resolve each selected URL before mounting the HTML reader: even a saved
  // feed entry may need ingestion, or may have a persisted PDF representation.
  // Route classification changes must not restart that same resolution.
  const resolveRef = useRef(resolve);
  useEffect(() => {
    resolveRef.current = resolve;
  }, [resolve]);
  const [resolvedUrl, setResolvedUrl] = useState<string | null>(null);
  useEffect(() => {
    abort();
    if (!url) return;
    const signal = abortControllerRef.current.signal;
    void resolveRef.current(url).finally(() => {
      if (!signal.aborted) setResolvedUrl(url);
    });
    return abort;
  }, [url, abort, abortControllerRef]);

  const [readerRevision, setReaderRevision] = useState(0);
  const articleReaderRef = useRef<ArticleReaderHandle>(null);

  const [reloading, setReloading] = useState(false);
  const reloadAttempt = useRef<symbol | null>(null);
  useEffect(() => {
    reloadAttempt.current = null;
    setReloading(false);
    return () => {
      reloadAttempt.current = null;
    };
  }, [url, entryId]);

  const handleReload = useCallback(async () => {
    if (reloadAttempt.current) return;
    const attempt = Symbol();
    reloadAttempt.current = attempt;
    setReloading(true);
    try {
      logger.debug("handleReload", entryId, url, articleReaderRef.current);
      if (url) {
        abort();
        const signal = abortControllerRef.current.signal;
        const succeeded = await resolve(url, true);
        if (succeeded && !signal.aborted && reloadAttempt.current === attempt)
          setReaderRevision((value) => value + 1);
      } else await articleReaderRef.current?.reload();
    } finally {
      if (reloadAttempt.current === attempt) {
        reloadAttempt.current = null;
        setReloading(false);
      }
    }
  }, [entryId, url, abort, resolve, abortControllerRef]);

  // The current URL's downloaded content was rejected.
  const blocked = !!url && blockedUrl === url;
  const isPdf = mimeType === "application/pdf" && !!url;
  // Wait for local lookup or ingestion before mounting a content reader.
  const isResolving = !!url && resolvedUrl !== url;

  return (
    <div ref={containerRef} className={cx(className, css.LayoutContainer)}>
      <div className={css.Layout}>
        <Toolbar
          reload={entryId || url ? handleReload : null}
          reloading={reloading}
          inspectorOpener={inspectorOpener}
          onOpenInspector={
            inspectorHidden && entryId && !isPdf
              ? () => {
                  inspectorReturnFocus.current = inspectorOpener.current;
                  setInspectorOpen(true);
                }
              : undefined
          }
          inspectorOpen={inspectorOpen}
        />

        <div className={css.ReaderAndInspector}>
          {isResolving ? (
            <div className={css.Placeholder}>Loading…</div>
          ) : blocked ? (
            <div className={css.Placeholder}>
              <Typography variant="body2" color="text.disabled">
                This download can’t be displayed in the reading pane because its
                format isn’t supported.
              </Typography>
            </div>
          ) : isPdf ? (
            <PdfReader src={pdfBlobUrl} />
          ) : entryId ? (
            <ArticleReader
              key={readerRevision}
              ref={articleReaderRef}
              onOpenInspector={openAnnotationInspector}
              inspectedAnnotationId={
                inspectorOpen ? drawerTarget?.annotationId : undefined
              }
            />
          ) : (
            <div className={css.Placeholder}>
              <Typography variant="body2" color="text.disabled">
                {isResolving ? "Loading…" : "Select an article"}
              </Typography>
            </div>
          )}
          {entryId && !isPdf && !blocked && (
            <aside
              ref={inspectorPanel}
              inert={inspectorHidden}
              aria-hidden={inspectorHidden}
              className={cx(css.InspectorPanel, inspectorHidden && css.InspectorPanelHidden)}
              aria-label="Article Inspector"
            >
              <div className={css.InspectorPanelContent}>
                <Inspector
                  target={{ entryId }}
                  view={inspectorView}
                  onViewChange={setInspectorView}
                />
              </div>
            </aside>
          )}
        </div>
        <Drawer
          data-inspector-drawer
          disableRestoreFocus
          anchor="right"
          open={
            useInspectorDrawer &&
            inspectorOpen &&
            !!entryId &&
            !isPdf &&
            !blocked
          }
          onClose={closeInspector}
          transitionDuration={animate ? undefined : 0}
          slotProps={{
            paper: {
              role: "dialog",
              "aria-modal": true,
              "aria-label": `${drawerTarget?.annotationId ? "Annotation" : "Article"} Inspector`,
              className: css.InspectorDrawer,
            },
            backdrop: { transitionDuration: animate ? undefined : 0 },
          }}
        >
          <div className={css.InspectorHeader}>
            <Typography component="h2" variant="subtitle2">
              {drawerTarget?.annotationId ? "Annotation" : "Article"} Inspector
            </Typography>
            <Tooltip title="Close Inspector">
              <IconButton aria-label="Close Inspector" onClick={closeInspector}>
                <Close />
              </IconButton>
            </Tooltip>
          </div>
          {entryId && drawerTarget && useInspectorDrawer && inspectorOpen && (
            <Inspector
              target={drawerTarget}
              showHeading={false}
              view={
                drawerTarget.annotationId
                  ? annotationInspectorView
                  : inspectorView
              }
              onViewChange={
                drawerTarget.annotationId
                  ? setAnnotationInspectorView
                  : setInspectorView
              }
            />
          )}
        </Drawer>
      </div>
    </div>
  );
}
