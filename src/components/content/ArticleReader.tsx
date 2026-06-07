import {
  Button,
  Collapse,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  LinearProgress,
  Typography,
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
import { useArticles } from "../../hooks/useArticles";
import { useEvents } from "../../hooks/useEvents";
import useRouter from "../../hooks/useRouter";
import {
  buildEntryHtml,
  type HtmlReaderStyle,
} from "../../text/buildEntryHtml";
import { markdownToHtml } from "../../text/markdownPipeline";
import type { Entry } from "../../types";
import cx from "../../utils/cx";
import { FONT_FAMILY_CSS } from "../../utils/FontFamily";
import Logger from "../../utils/Logger";
import css from "./ArticleReader.module.css";
import { HtmlReader } from "./HtmlReader";
import { RelatedItemList } from "./RelatedItemList";

export interface ArticleReaderHandle {
  reload: () => void;
}

export interface ArticleReaderProps {
  className?: string;
}

const logger = Logger.getLogger("ArticleReader");

const ArticleReader = forwardRef<ArticleReaderHandle, ArticleReaderProps>(
  function ArticleReader({ className }, ref) {
    const { selectUrl, replaceEntry, replaceUrl, state } = useRouter();
    const { entryId, annotationId, url } = state;

    const {
      htmlReaderView,
      themeMode,
      htmlReaderFontFamily,
      htmlReaderFontSize,
      htmlReaderLineHeight,
    } = useAppearance();

    const [abort, abortControllerRef] = useAbortController();
    const [title, setTitle] = useState<string | null>(null);
    const [summaryHtml, setSummaryHtml] = useState<string | null>(null);
    const [contentHtml, setContentHtml] = useState<string | null>(null);
    // The Entry currently loaded into the reader. Drives the replaceEntry effect
    // below, which reconciles the route's entryId when a followed link cleared
    // it (whether the target was freshly fetched or already saved).
    const [loadedEntry, setLoadedEntry] = useState<Entry | null>(null);

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
      loadArticleByUrl,
      findArticleByUrl,
      extractArticleFromHtml,
      saveExtractedArticle,
    } = useArticles(abortControllerRef);

    const [pendingUpdate, setPendingUpdate] = useState<{
      title: string;
      markdown: string;
      favicon: string | null;
    } | null>(null);

    const requestContent = useCallback(
      async (reload = false) => {
        logger.info("requestContent", url);
        if (!url) return;
        setTitle(null);
        setSummaryHtml(null);
        setContentHtml(null);
        setLoadedEntry(null);

        // A saved article (selected Entry). For a link followed within the
        // reader there may be no saved article yet, so fall back to fetching the
        // remote content by URL, which also persists it as a new article.
        const article =
          (await findArticleByUrl(url)) ??
          (await loadArticleByUrl(url))?.article;
        if (!article) return;

        // Record the loaded Entry so the effect below can reconcile the route's
        // entryId with it. This covers both a freshly fetched article and a
        // previously saved one reached via a link (which cleared the entryId).
        setLoadedEntry(article);

        setTitle(article.title ?? "Untitled");

        if (article.summary) {
          const html = await markdownToHtml(article.summary);
          setSummaryHtml(buildEntryHtml(html, htmlReaderStyle));
        }

        setAnnotationsFromArticle(article);

        if (!article.content || reload) {
          const content = (await loadArticle(article))?.content;
          if (content) article.content = content;
        }

        const html = await markdownToHtml(article.content);
        setContentHtml(buildEntryHtml(html, htmlReaderStyle));
      },
      [
        findArticleByUrl,
        htmlReaderStyle,
        loadArticle,
        loadArticleByUrl,
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

      if (article.content) {
        const html = await markdownToHtml(article.content);
        if (html) setContentHtml(buildEntryHtml(html, htmlReaderStyle));
      }
    }, [
      entryId,
      htmlReaderStyle,
      pendingUpdate,
      readEntry,
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
      // eslint-disable-next-line react-hooks/set-state-in-effect
      requestContent();
      return abort;
    }, [abort, requestContent]);

    // When a link followed within the reader is loaded by URL, the route lacks
    // its entryId. Reflect it here rather than in requestContent so the route
    // patch does not re-trigger a content reload. Kept off requestContent's deps
    // for the same reason.
    useEffect(() => {
      if (loadedEntry && loadedEntry.id !== entryId) {
        replaceEntry(loadedEntry);
      }
    }, [loadedEntry, entryId, replaceEntry]);

    useEffect(() => {
      if (!annotationId) return;
      if (!entryId) return;

      (async () => {
        const entry = await readEntry(entryId);
        if (entry) replaceUrl(entry.url);
      })();
    }, [annotationId, entryId, readEntry, replaceUrl, selectUrl]);

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
        async ({ url: loaderUrl, html: loaderHtml, mimeType }) => {
          if (url !== loaderUrl) return;
          if (mimeType === "application/pdf") return;

          const extracted = await extractArticleFromHtml(url, loaderHtml);
          if (!extracted?.markdown) return;

          setPendingUpdate(extracted);
        }
      );
    }, [onLoaderClosed, extractArticleFromHtml, url]);

    return (
      <>
        <div className={css.Content}>
          <div className={css.Entry}>
            <div className={css.Title}>
              <Typography
                component="h1"
                variant="h5"
                sx={{ fontFamily: FONT_FAMILY_CSS[htmlReaderFontFamily] }}
              >
                {title ?? <>Loading...</>}
              </Typography>
            </div>

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
                onSaveAnnotation={saveAnnotation}
                onRemoveAnnotation={removeAnnotation}
              />
            )}
          </div>

          {entryId && (
            <aside className={css.Aside}>
              <RelatedItemList itemId={entryId} />
            </aside>
          )}
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
