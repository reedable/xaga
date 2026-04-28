import { Bookmark, MoreVert, OpenInNew, Refresh } from "@mui/icons-material";
import {
  ButtonGroup,
  Collapse,
  IconButton,
  LinearProgress,
  Menu,
  MenuItem,
  Tooltip,
  Typography,
} from "@mui/material";
import { useCallback, useEffect, useMemo, useState } from "react";
import useAbortController from "../../hooks/useAbortController";
import { useAnnotations } from "../../hooks/useAnnotations";
import { useAppearance } from "../../hooks/useAppearance";
import { useArticles } from "../../hooks/useArticles";
import { useBookmarks } from "../../hooks/useBookmarks";
import useEntries from "../../hooks/useEntries";
import { useEvents } from "../../hooks/useEvents";
import { usePlatform } from "../../platform/Platform";
import { buildEntryHtml, type ReaderStyle } from "../../text/buildEntryHtml";
import { markdownToHtml } from "../../text/markdownPipeline";
import { type Entry, type Item } from "../../types";
import cx from "../../utils/cx";
import { FONT_FAMILY_CSS } from "../../utils/FontFamily";
import formatByline from "../../utils/formatByline";
import css from "./_.module.css";
import { Reader } from "./Reader";
import { RelatedItemList } from "./RelatedItemList";

export interface ArticleReaderProps {
  className?: string;
  selectedEntry: Entry;
  onChangeEntry?: (entry: Item) => void;
  onSelectEntry: (entry: Item) => void;
  setLoading?: (loading: boolean) => void;
}

export default function ArticleReader({
  className,
  selectedEntry,
  onChangeEntry,
  onSelectEntry,
  setLoading,
}: ArticleReaderProps) {
  const platform = usePlatform();

  const {
    readerView,
    themeMode,
    readerFontFamily,
    readerFontSize,
    readerLineHeight,
  } = useAppearance();

  const [abort, abortControllerRef] = useAbortController();
  const [bodyHtml, setBodyHtml] = useState<string | null>(null);
  const [menuAnchor, setMenuAnchor] = useState<HTMLElement | null>(null);

  // Annotations
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
    [themeMode, readerFontFamily, readerFontSize, readerLineHeight],
  );

  const html = useMemo(
    () => (bodyHtml ? buildEntryHtml(bodyHtml, readerStyle) : null),
    [bodyHtml, readerStyle],
  );

  const { addBookmark, removeBookmark } = useBookmarks(abortControllerRef);
  const { markEntryRead, markEntryUnread } = useEntries(abortControllerRef);
  const { onUnreadChanged, onBookmarkChanged } = useEvents();

  useEffect(() => {
    return onUnreadChanged((entry) => {
      if (entry.id === selectedEntry.id) onChangeEntry?.(entry);
    });
  }, [onUnreadChanged, selectedEntry.id, onChangeEntry]);

  useEffect(() => {
    return onBookmarkChanged((entry) => {
      if (entry.id === selectedEntry.id) onChangeEntry?.(entry);
    });
  }, [onBookmarkChanged, selectedEntry.id, onChangeEntry]);

  const {
    loading,
    readArticle,
    loadArticle,
    findArticleByUrl,
    loadArticleByUrl,
  } = useArticles(abortControllerRef);

  const requestArticle = useCallback(
    async (entry: Entry) => {
      setBodyHtml(null);

      if (readerView === "full") {
        const article = await readArticle(entry);
        if (article) setAnnotationsFromArticle(article);
        const markdown = article?.content || article?.summary;

        if (markdown) {
          const body = await markdownToHtml(markdown);
          if (body) setBodyHtml(body);
        }
      } else {
        if (entry.summary) {
          const body = await markdownToHtml(entry.summary);
          if (body) setBodyHtml(body);
        }

        await markEntryRead(entry);
      }
    },
    [readerView, readArticle, setAnnotationsFromArticle, markEntryRead],
  );

  const reloadArticle = useCallback(
    async (entry: Entry) => {
      setBodyHtml(null);

      const article = await loadArticle(entry);

      if (article?.content) {
        const body = await markdownToHtml(article.content);
        if (body) setBodyHtml(body);
      }
    },
    [loadArticle],
  );

  const handleOpenUrl = useCallback(() => {
    platform.openUrl(selectedEntry.url);
  }, [platform, selectedEntry]);

  const toggleBookmark = useCallback(async () => {
    if (selectedEntry.isBookmarked) {
      await removeBookmark(selectedEntry);
    } else {
      await addBookmark(selectedEntry);
    }
  }, [addBookmark, removeBookmark, selectedEntry]);

  const handleLoadArticleByUrl = useCallback(
    async (url: string) => {
      setBodyHtml(null);

      let article = await findArticleByUrl(url);

      if (!article) {
        article = await loadArticleByUrl(url);
      }

      if (article?.content) {
        const body = await markdownToHtml(article.content);
        if (body) setBodyHtml(body);
      }

      if (article) onSelectEntry(article);
    },
    [findArticleByUrl, loadArticleByUrl, onSelectEntry],
  );

  const handleMarkRead = useCallback(async () => {
    await markEntryRead(selectedEntry);
    setMenuAnchor(null);
  }, [markEntryRead, selectedEntry]);

  const handleMarkUnread = useCallback(async () => {
    await markEntryUnread(selectedEntry);
    setMenuAnchor(null);
  }, [markEntryUnread, selectedEntry]);

  const handleReload = useCallback(async () => {
    reloadArticle(selectedEntry);
    setMenuAnchor(null);
  }, [reloadArticle, selectedEntry]);

  useEffect(() => {
    abort();
    (async () => {
      requestArticle(selectedEntry);
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [requestArticle, abort, selectedEntry.id, readerView]);

  useEffect(() => {
    setLoading?.(loading);
  }, [loading, setLoading]);

  return (
    <>
      <div className={css.Header}>
        <div className={css.Title}>
          <Typography component="h1" variant="h5" sx={{ fontFamily: FONT_FAMILY_CSS[readerFontFamily] }}>
            {selectedEntry.title}
          </Typography>
        </div>
      </div>

      <div className={css.Content}>
        <div className={css.Entry}>
          <div className={css.Byline}>
            <Typography variant="caption" color="text.secondary">
              {formatByline(selectedEntry)}
            </Typography>

            <ButtonGroup>
              <Tooltip
                title={
                  selectedEntry.isBookmarked
                    ? "Remove bookmark"
                    : "Add bookmark"
                }
              >
                <IconButton size="small" onClick={toggleBookmark}>
                  <Bookmark
                    color={
                      selectedEntry.isBookmarked ? "primary" : undefined
                    }
                  />
                </IconButton>
              </Tooltip>

              <Tooltip title="Open in browser">
                <IconButton size="small" onClick={handleOpenUrl}>
                  <OpenInNew />
                </IconButton>
              </Tooltip>

              <Tooltip title="More actions">
                <IconButton
                  size="small"
                  onClick={(e) => setMenuAnchor(e.currentTarget)}
                >
                  <MoreVert fontSize="small" />
                </IconButton>
              </Tooltip>
            </ButtonGroup>

            <Menu
              anchorEl={menuAnchor}
              open={Boolean(menuAnchor)}
              onClose={() => setMenuAnchor(null)}
            >
              {selectedEntry.isUnread ? (
                <MenuItem onClick={handleMarkRead}>Mark as read</MenuItem>
              ) : (
                <MenuItem onClick={handleMarkUnread}>Mark as unread</MenuItem>
              )}

              <MenuItem onClick={handleReload}>
                <Refresh fontSize="small" sx={{ mr: 1 }} />
                Reload document
              </MenuItem>
            </Menu>
          </div>

          <Collapse in={loading} unmountOnExit>
            <LinearProgress />

            <div className={css.Advisory}>
              <Typography variant="caption">Loading...</Typography>
            </div>
          </Collapse>

          {html && (
            <Reader
              className={cx(className)}
              html={html}
              reference={selectedEntry.id}
              annotations={annotations}
              onClickUrl={handleLoadArticleByUrl}
              onSaveAnnotation={saveAnnotation}
              onRemoveAnnotation={removeAnnotation}
            />
          )}
        </div>

        <aside>
          <RelatedItemList
            selectedItem={selectedEntry}
            onSelectItem={onSelectEntry}
          />
        </aside>
      </div>
    </>
  );
}
