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
import { useContentLoader } from "../../hooks/useContentLoader";
import { useEvents } from "../../hooks/useEvents";
import useRouter from "../../hooks/useRouter";
import {
  buildEntryHtml,
  type HtmlReaderStyle,
} from "../../text/buildEntryHtml";
import { markdownToHtml } from "../../text/markdownPipeline";
import { type Article, type Entry } from "../../types";
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
    const { show, openUrl, state } = useRouter();
    const { entryId, annotationId, url } = state;

    const {
      htmlReaderView,
      themeMode,
      htmlReaderFontFamily,
      htmlReaderFontSize,
      htmlReaderLineHeight,
    } = useAppearance();

    const [abort, abortControllerRef] = useAbortController();
    const [selectedEntry, setSelectedEntry] = useState<Entry | null>(null);
    const [selectedArticle, setSelectedArticle] = useState<Article | null>(
      null
    );
    const [html, setHtml] = useState<string | null>(null);

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

    const entryHtml = useMemo(
      () => (html ? buildEntryHtml(html, htmlReaderStyle) : null),
      [html, htmlReaderStyle]
    );

    const {
      setFindEnabled,
      onUnreadChanged,
      onBookmarkChanged,
      onLoaderClosed,
      onLoaderPageLoaded,
    } = useEvents();

    const {
      loading,
      readEntry,
      readArticle,
      loadArticle,
      findArticleByUrl,
      createArticleFromHtml,
      extractArticleFromHtml,
      saveExtractedArticle,
    } = useArticles(abortControllerRef);

    const { loadUrl } = useContentLoader(abortControllerRef);

    const [pendingUpdate, setPendingUpdate] = useState<{
      title: string;
      markdown: string;
      favicon: string | null;
    } | null>(null);

    // Applies a loaded Article to display state.
    const showArticle = useCallback(
      async (article: Article) => {
        setAnnotationsFromArticle(article);
        setHtml((await markdownToHtml(article.content)) ?? null);
      },
      [setAnnotationsFromArticle]
    );

    // Effect 1 — fires when the selected entry changes.
    useEffect(() => {
      if (!entryId) return;
      abort();

      (async () => {
        setSelectedEntry(null);
        setSelectedArticle(null);
        setHtml(null);

        const entry = await readEntry(entryId);
        if (!entry) return;
        setSelectedEntry(entry);

        if (htmlReaderView === "summary") {
          setHtml(
            entry.summary
              ? ((await markdownToHtml(entry.summary)) ?? null)
              : null
          );
        } else {
          if (entry.summary)
            setHtml((await markdownToHtml(entry.summary)) ?? null);
          const article = await readArticle(entryId);
          if (article) {
            setSelectedArticle(article);
            await showArticle(article);
          } else if (!entry.summary) {
            setHtml(null);
          }
        }
      })();

      return abort;
      // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [entryId, annotationId, abort]);

    // Effect 2 — fires when the view mode changes; re-renders from loaded data.
    useEffect(() => {
      if (!selectedEntry) return;

      (async () => {
        if (htmlReaderView === "summary") {
          setHtml(
            selectedEntry.summary
              ? ((await markdownToHtml(selectedEntry.summary)) ?? null)
              : null
          );
        } else if (selectedArticle) {
          await showArticle(selectedArticle);
        } else {
          if (selectedEntry.summary)
            setHtml((await markdownToHtml(selectedEntry.summary)) ?? null);
          const article = await readArticle(selectedEntry.id);
          if (article) {
            setSelectedArticle(article);
            await showArticle(article);
          }
        }
      })();
      // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [htmlReaderView, selectedEntry]);

    const reloadArticle = useCallback(
      async (entry: Entry) => {
        logger.debug("reloadArticle", entry);
        setHtml(null);

        const article = await loadArticle(entry);

        if (article?.content) {
          setSelectedArticle(article);
          await showArticle(article);
        }
      },
      [loadArticle, showArticle]
    );

    const handleLoadArticleByUrl = useCallback(
      async (clickedUrl: string) => {
        logger.debug("reloadArticleByUrl", clickedUrl);
        setSelectedEntry(null);
        setSelectedArticle(null);
        setHtml(null);
        openUrl(clickedUrl);
        await loadUrl(clickedUrl);
      },
      [loadUrl, openUrl]
    );

    useImperativeHandle(
      ref,
      () => ({
        reload: () => {
          logger.debug("reload", url, selectedEntry);

          if (url) {
            handleLoadArticleByUrl(url);
          } else if (selectedEntry) {
            reloadArticle(selectedEntry);
          }
        },
      }),
      [selectedEntry, reloadArticle, url, handleLoadArticleByUrl]
    );

    useEffect(() => {
      setFindEnabled(!!entryHtml);
      return () => {
        setFindEnabled(false);
      };
    }, [entryHtml, setFindEnabled]);

    useEffect(() => {
      return onUnreadChanged((entry) => {
        setSelectedEntry((prev) =>
          prev && prev.id === entry.id ? { ...prev, ...entry } : prev
        );
      });
    }, [onUnreadChanged]);

    useEffect(() => {
      return onBookmarkChanged((entry) => {
        setSelectedEntry((prev) =>
          prev && prev.id === entry.id ? { ...prev, ...entry } : prev
        );
      });
    }, [onBookmarkChanged]);

    useEffect(() => {
      if (!selectedEntry) return;
      return onLoaderClosed(
        async ({ url: loaderUrl, html: loaderHtml, mimeType }) => {
          if (selectedEntry.url !== loaderUrl) return;
          if (mimeType === "application/pdf") return;
          const extracted = await extractArticleFromHtml(
            selectedEntry.url,
            loaderHtml
          );
          if (!extracted?.markdown) return;
          if (
            extracted.markdown.trim() ===
            (selectedArticle?.content ?? "").trim()
          ) {
            return;
          }
          setPendingUpdate(extracted);
        }
      );
    }, [
      onLoaderClosed,
      selectedEntry,
      selectedArticle,
      extractArticleFromHtml,
    ]);

    useEffect(() => {
      return onLoaderPageLoaded(
        async ({ url: newUrl, html: loaderHtml, mimeType }) => {
          if (mimeType === "application/pdf") {
            openUrl(newUrl);
            await loadUrl(newUrl);
            return;
          }
          let article = await findArticleByUrl(newUrl);
          if (!article)
            article = await createArticleFromHtml(newUrl, loaderHtml);
          if (!article) return;
          show(article);
        }
      );
    }, [
      onLoaderPageLoaded,
      openUrl,
      loadUrl,
      findArticleByUrl,
      createArticleFromHtml,
      show,
    ]);

    const handleConfirmUpdate = useCallback(async () => {
      if (!pendingUpdate || !selectedEntry) return;
      setPendingUpdate(null);
      const article = await saveExtractedArticle(selectedEntry, pendingUpdate);
      if (!article) return;
      setSelectedArticle(article);
      setAnnotationsFromArticle(article);
      const body = await markdownToHtml(
        article.content || article.summary || ""
      );
      if (body) setHtml(body);
    }, [
      pendingUpdate,
      selectedEntry,
      saveExtractedArticle,
      setAnnotationsFromArticle,
    ]);

    const placeholderText =
      selectedEntry && !loading && !entryHtml
        ? htmlReaderView === "summary"
          ? "No summary available."
          : "No content available."
        : null;

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
                {selectedEntry?.title || <>Loading...</>}
              </Typography>
            </div>

            <div
              className={cx(
                css.ProgressBar,
                entryHtml && css.ProgressBarLoaded
              )}
            >
              <Collapse in={Boolean(url && loading)}>
                <LinearProgress variant="indeterminate" />
              </Collapse>
            </div>

            {placeholderText && (
              <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
                {placeholderText}
              </Typography>
            )}

            {selectedEntry && entryHtml && (
              <HtmlReader
                className={cx(className)}
                html={entryHtml}
                entryId={selectedEntry.id}
                annotations={annotations}
                scrollToAnnotationId={annotationId}
                onClickLink={handleLoadArticleByUrl}
                onSaveAnnotation={saveAnnotation}
                onRemoveAnnotation={removeAnnotation}
              />
            )}
          </div>

          {selectedEntry && (
            <aside className={css.Aside}>
              <RelatedItemList itemId={selectedEntry.id} />
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
