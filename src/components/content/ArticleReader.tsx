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
import { buildEntryHtml, type HtmlReaderStyle } from "../../text/buildEntryHtml";
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
    const [selectedEntry, setSelectedEntry] = useState<Article | null>(null);
    const [bodyHtml, setBodyHtml] = useState<string | null>(null);

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
      [themeMode, htmlReaderFontFamily, htmlReaderFontSize, htmlReaderLineHeight]
    );

    const html = useMemo(
      () => (bodyHtml ? buildEntryHtml(bodyHtml, htmlReaderStyle) : null),
      [bodyHtml, htmlReaderStyle]
    );

    const {
      setFindEnabled,
      onUnreadChanged,
      onBookmarkChanged,
      onLoaderClosed,
      onLoaderPageLoaded,
    } = useEvents();

    const {
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

    const requestArticle = useCallback(
      async (id: Entry["id"]) => {
        setSelectedEntry(null);
        setBodyHtml(null);

        const article = await readArticle(id);

        if (article) {
          setSelectedEntry(article);
          setAnnotationsFromArticle(article);
        }

        const markdown = article?.content || article?.summary;

        if (markdown) {
          const body = await markdownToHtml(markdown);
          if (body) setBodyHtml(body);
        }
      },
      [readArticle, setAnnotationsFromArticle]
    );

    const reloadArticle = useCallback(
      async (entry: Entry) => {
        logger.debug("reloadArticle", entry);
        setBodyHtml(null);

        const article = await loadArticle(entry);

        if (article?.content) {
          const body = await markdownToHtml(article.content);
          if (body) setBodyHtml(body);
        }
      },
      [loadArticle]
    );

    const handleLoadArticleByUrl = useCallback(
      async (clickedUrl: string) => {
        logger.debug("reloadArticleByUrl", clickedUrl);
        setSelectedEntry(null);
        setBodyHtml(null);
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
      setFindEnabled(!!html);
      return () => {
        setFindEnabled(false);
      };
    }, [html, setFindEnabled]);

    useEffect(() => {
      if (!entryId) return;
      abort();

      (async () => {
        requestArticle(entryId);
      })();

      return abort;
    }, [requestArticle, abort, entryId, htmlReaderView, annotationId]);

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
      return onLoaderClosed(async ({ url, html, mimeType }) => {
        if (selectedEntry.url !== url) return;
        if (mimeType === "application/pdf") return;
        const extracted = await extractArticleFromHtml(selectedEntry.url, html);
        if (!extracted?.markdown) return;
        if (
          extracted.markdown.trim() === (selectedEntry.content ?? "").trim()
        ) {
          return;
        }
        setPendingUpdate(extracted);
      });
    }, [onLoaderClosed, selectedEntry, extractArticleFromHtml]);

    useEffect(() => {
      return onLoaderPageLoaded(async ({ url: newUrl, html, mimeType }) => {
        if (mimeType === "application/pdf") {
          openUrl(newUrl);
          await loadUrl(newUrl);
          return;
        }
        let article = await findArticleByUrl(newUrl);
        if (!article) article = await createArticleFromHtml(newUrl, html);
        if (!article) return;
        show(article);
      });
    }, [onLoaderPageLoaded, openUrl, loadUrl, findArticleByUrl, createArticleFromHtml, show]);

    const handleConfirmUpdate = useCallback(async () => {
      if (!pendingUpdate || !selectedEntry) return;
      setPendingUpdate(null);
      const article = await saveExtractedArticle(selectedEntry, pendingUpdate);
      if (!article) return;
      setSelectedEntry(article);
      setAnnotationsFromArticle(article);
      const markdown = article.content || article.summary;
      if (markdown) {
        const body = await markdownToHtml(markdown);
        if (body) setBodyHtml(body);
      }
    }, [
      pendingUpdate,
      selectedEntry,
      saveExtractedArticle,
      setAnnotationsFromArticle,
    ]);

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

            <div className={cx(css.ProgressBar, html && css.ProgressBarLoaded)}>
              <Collapse in={Boolean(url && !html)}>
                <LinearProgress variant="indeterminate" />
              </Collapse>
            </div>

            {selectedEntry && html && (
              <HtmlReader
                className={cx(className)}
                html={html}
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

        <Dialog open={pendingUpdate !== null} onClose={() => setPendingUpdate(null)}>
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
