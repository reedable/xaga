import { Typography } from "@mui/material";
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
import { buildEntryHtml, type ReaderStyle } from "../../text/buildEntryHtml";
import { markdownToHtml } from "../../text/markdownPipeline";
import { type Article, type Entry } from "../../types";
import cx from "../../utils/cx";
import { FONT_FAMILY_CSS } from "../../utils/FontFamily";
import Logger from "../../utils/Logger";
import css from "./ArticleReader.module.css";
import { Reader } from "./Reader";
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
      readerView,
      themeMode,
      readerFontFamily,
      readerFontSize,
      readerLineHeight,
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

    const readerStyle: ReaderStyle = useMemo(
      () => ({
        themeMode,
        fontFamily: readerFontFamily,
        fontSize: readerFontSize,
        lineHeight: readerLineHeight,
      }),
      [themeMode, readerFontFamily, readerFontSize, readerLineHeight]
    );

    const html = useMemo(
      () => (bodyHtml ? buildEntryHtml(bodyHtml, readerStyle) : null),
      [bodyHtml, readerStyle]
    );

    const { setFindEnabled, onUnreadChanged, onBookmarkChanged } = useEvents();

    const { readArticle, loadArticle, findArticleByUrl, loadArticleByUrl } =
      useArticles(abortControllerRef);

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
        setBodyHtml(null);
        openUrl(clickedUrl);

        let article = await findArticleByUrl(clickedUrl);
        if (!article) article = await loadArticleByUrl(clickedUrl);

        if (article?.content) {
          const body = await markdownToHtml(article.content);
          if (body) setBodyHtml(body);
        }

        if (article) show(article);
      },
      [findArticleByUrl, loadArticleByUrl, openUrl, show]
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
    }, [requestArticle, abort, entryId, readerView, annotationId]);

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

    return (
      <>
        <div className={css.Content}>
          <div className={css.Entry}>
            <div className={css.Title}>
              <Typography
                component="h1"
                variant="h5"
                sx={{ fontFamily: FONT_FAMILY_CSS[readerFontFamily] }}
              >
                {selectedEntry?.title || <>&nbsp;</>}
              </Typography>
            </div>

            {selectedEntry && html && (
              <Reader
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
      </>
    );
  }
);

export default ArticleReader;
