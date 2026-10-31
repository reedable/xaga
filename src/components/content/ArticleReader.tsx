import {
  Button,
  Collapse,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  LinearProgress,
} from "@mui/material";
import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useMemo,
  useState,
} from "react";
import useAbortController from "../../hooks/useAbortController";
import { useAnnotations } from "../../hooks/useAnnotations";
import { useAppearance } from "../../hooks/useAppearance";
import { toLoaderFavicon, useArticles } from "../../hooks/useArticles";
import { useErrorSink } from "../../hooks/useErrorSink";
import { useEvents } from "../../hooks/useEvents";
import useRouter from "../../hooks/useRouter";
import { usePlatform } from "../../platform/Platform";
import {
  buildEntryHtml,
  type HtmlReaderStyle,
} from "../../text/buildEntryHtml";
import { prepareEntryImages } from "../../text/prepareEntryImages";
import { imageReferences } from "../../text/imageReferences";
import { markdownToHtml } from "../../text/markdownPipeline";
import { type Annotation, isAbortError } from "../../types";
import ArticleLoadAttempt from "../../utils/ArticleLoadAttempt";
import cx from "../../utils/cx";
import css from "./ArticleReader.module.css";
import { HtmlReader } from "./HtmlReader";

export interface ArticleReaderHandle {
  reload: () => Promise<void>;
}

interface ArticleReaderProps {
  className?: string;
  onOpenInspector?: (
    annotation: Pick<Annotation, "id" | "reference">,
    opener?: HTMLElement
  ) => void;
  inspectedAnnotationId?: string;
}

const ArticleReader = forwardRef<ArticleReaderHandle, ArticleReaderProps>(
  function ArticleReader(
    { className, onOpenInspector, inspectedAnnotationId },
    ref
  ) {
    const { selectUrl, replaceUrl, state } = useRouter();
    const { entryId, annotationId, url } = state;

    const {
      htmlReaderView,
      themeMode,
      htmlReaderFontFamily,
      htmlReaderFontSize,
      htmlReaderLineHeight,
    } = useAppearance();

    const platform = usePlatform();
    const { pushError } = useErrorSink();
    const [abort, abortControllerRef] = useAbortController();
    const [summaryHtml, setSummaryHtml] = useState<string | null>(null);
    const [contentHtml, setContentHtml] = useState<string | null>(null);

    const html = useMemo(() => {
      if (htmlReaderView === "summary") {
        return summaryHtml ?? "";
      }
      return (contentHtml || summaryHtml) ?? "";
    }, [contentHtml, htmlReaderView, summaryHtml]);

    const {
      annotations,
      setAnnotationsFromArticle,
      saveAnnotation,
      replaceAnnotation,
      removeAnnotation,
    } = useAnnotations(abortControllerRef);

    const htmlReaderStyle: HtmlReaderStyle = useMemo(
      () => ({
        themeMode,
        fontFamily: htmlReaderFontFamily,
        fontSize: htmlReaderFontSize,
        lineHeight: htmlReaderLineHeight,
      }),
      [
        themeMode,
        htmlReaderFontFamily,
        htmlReaderFontSize,
        htmlReaderLineHeight,
      ]
    );

    // Resolve an entry's stored `../images/…` references to data: URIs at render.
    const resolveEntryImages = useCallback(
      (id: string, refs: string[]) =>
        platform.readEntryImages(abortControllerRef.current.signal, id, refs),
      [platform, abortControllerRef]
    );

    const {
      setFindEnabled,
      onUnreadChanged,
      onBookmarkChanged,
      onLoaderClosed,
    } = useEvents();

    const {
      loading,
      readEntry,
      loadArticle,
      findArticleByUrl,
      extractArticleFromHtml,
      saveExtractedArticle,
    } = useArticles(abortControllerRef);

    const [pendingUpdate, setPendingUpdate] = useState<{
      title: string;
      markdown: string;
      favicon: string | null;
      imageData?: Record<string, string>;
    } | null>(null);

    // Reads and renders the already-resolved article for the current route.
    // useContentLoader has classified the URL and persisted any new article
    // before navigating here, so this only reads + renders (and reloads the
    // body from the network only when an explicit reload was requested).
    const requestContent = useCallback(
      async (reload = false) => {
        if (!url) return;
        const signal = abortControllerRef.current.signal;
        if (!reload) {
          setSummaryHtml(null);
          setContentHtml(null);
        }

        const attempt = await ArticleLoadAttempt.create(
          platform,
          reload ? "refresh" : "render",
          signal
        );
        let outcome: "succeeded" | "failed" | "cancelled" = "succeeded";
        try {
          const article = await findArticleByUrl(url, attempt);
          if (!article) return;
          signal.throwIfAborted();

          const title = article.title ?? "Untitled";

          setAnnotationsFromArticle(article);
          if (reload) {
            const content = (await loadArticle(article, false, attempt))
              ?.content;
            if (content) article.content = content;
          }
          const render = (value: string) =>
            attempt.runStep("render", async () =>
              buildEntryHtml(
                await prepareEntryImages(
                  await markdownToHtml(value),
                  article.id,
                  (id, refs) => platform.readEntryImages(signal, id, refs)
                ),
                title,
                htmlReaderStyle
              )
            );
          // First display available local content, without waiting for remote recovery.
          const summaryHtml = article.description
            ? await render(article.description)
            : null;
          const contentHtml = await render(article.content);
          signal.throwIfAborted();
          setSummaryHtml(summaryHtml);
          setContentHtml(contentHtml);
          const enabled = await platform.getLoadRemoteImage();
          signal.throwIfAborted();
          if (enabled) {
            const summary = htmlReaderView === "summary" || !article.content;
            const source = summary
              ? (article.description ?? "")
              : article.content;
            const patches = imageReferences(source, url);
            if (!patches.length) return;
            const updated = await attempt.runStep("images", () =>
              platform.recoverEntryImages(
                signal,
                article.id,
                { summary, expected: source, patches },
                attempt.id
              )
            );
            signal.throwIfAborted();
            const html = summary
              ? updated.description
                ? await render(updated.description)
                : null
              : await render(updated.content);
            signal.throwIfAborted();
            if (summary) setSummaryHtml(html);
            else setContentHtml(html);
          }
        } catch (error) {
          outcome = isAbortError(error) ? "cancelled" : "failed";
          pushError(error);
        } finally {
          await attempt.complete(outcome);
        }
      },
      [
        platform,
        pushError,
        abortControllerRef,
        findArticleByUrl,
        htmlReaderStyle,
        htmlReaderView,
        loadArticle,
        setAnnotationsFromArticle,
        url,
      ]
    );

    const handleConfirmUpdate = useCallback(async () => {
      if (!entryId) return;
      if (!pendingUpdate) return;

      const entry = await readEntry(entryId);
      if (!entry) return;

      const article = await saveExtractedArticle(entry, pendingUpdate);
      if (!article) return;

      setPendingUpdate(null);
      setAnnotationsFromArticle(article);

      const title = article.title ?? "Untitled";
      const render = async (markdown: string) =>
        buildEntryHtml(
          await prepareEntryImages(
            await markdownToHtml(markdown),
            article.id,
            resolveEntryImages
          ),
          title,
          htmlReaderStyle
        );
      setSummaryHtml(
        article.description ? await render(article.description) : null
      );
      setContentHtml(await render(article.content));
    }, [
      entryId,
      htmlReaderStyle,
      pendingUpdate,
      readEntry,
      resolveEntryImages,
      saveExtractedArticle,
      setAnnotationsFromArticle,
    ]);

    useImperativeHandle(
      ref,
      () => ({
        reload: () => requestContent(true),
      }),
      [requestContent]
    );

    useEffect(() => {
      abort();
      requestContent();
      return abort;
    }, [abort, requestContent]);

    // An annotation selected from a list carries entryId but no url, which
    // requestContent needs to read the article. Backfill it onto the route.
    useEffect(() => {
      if (!annotationId) return;
      if (!entryId) return;
      if (url) return;

      (async () => {
        const entry = await readEntry(entryId);
        if (entry) replaceUrl(entry.url);
      })();
    }, [annotationId, entryId, url, readEntry, replaceUrl]);

    useEffect(() => {
      setFindEnabled(!!(summaryHtml || contentHtml));

      return () => {
        setFindEnabled(false);
      };
    }, [summaryHtml, contentHtml, setFindEnabled]);

    useEffect(() => {
      return onUnreadChanged((entry) => {
        console.log("onUnreadChanged", entry);
      });
    }, [onUnreadChanged]);

    useEffect(() => {
      return onBookmarkChanged((entry) => {
        // TODO Implement me
        console.log("onBookmarkChanged", entry);
      });
    }, [onBookmarkChanged]);

    useEffect(() => {
      return onLoaderClosed(
        async ({
          loaderUrl,
          samePage,
          html: loaderHtml,
          mimeType,
          favicon,
          faviconData,
        }) => {
          if (url !== loaderUrl) return;
          // The captured html belongs to pageUrl; when the loader was
          // redirected or navigated to a different page, it must not be
          // offered as an update for this article.
          if (!samePage) return;
          if (mimeType === "application/pdf") return;

          const extracted = await extractArticleFromHtml(
            url,
            loaderHtml,
            toLoaderFavicon(favicon, faviconData)
          );
          if (!extracted?.markdown) return;

          setPendingUpdate(extracted);
        }
      );
    }, [onLoaderClosed, extractArticleFromHtml, url]);

    return (
      <>
        <div className={css.Content}>
          <div className={css.Entry}>
            <div
              className={cx(
                css.ProgressBar,
                contentHtml && css.ProgressBarLoaded
              )}
            >
              <Collapse in={Boolean(url && loading)}>
                <LinearProgress variant="indeterminate" />
              </Collapse>
            </div>

            {(summaryHtml || contentHtml) && (
              <HtmlReader
                className={cx(className)}
                html={html}
                entryId={entryId}
                annotations={annotations}
                annotationId={annotationId}
                onClickLink={selectUrl}
                onOpenInspector={onOpenInspector}
                inspectedAnnotationId={inspectedAnnotationId}
                onSaveAnnotation={saveAnnotation}
                onReplaceAnnotation={replaceAnnotation}
                onRemoveAnnotation={removeAnnotation}
              />
            )}
          </div>
        </div>

        <Dialog
          open={pendingUpdate !== null}
          onClose={() => setPendingUpdate(null)}
        >
          <DialogTitle>Update article?</DialogTitle>
          <DialogContent>
            <DialogContentText>
              The page content differs from your saved version.
            </DialogContentText>
          </DialogContent>
          <DialogActions>
            <Button onClick={() => setPendingUpdate(null)}>Keep old</Button>
            <Button onClick={handleConfirmUpdate} variant="contained">
              Update
            </Button>
          </DialogActions>
        </Dialog>
      </>
    );
  }
);

export default ArticleReader;
