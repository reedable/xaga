import { Bookmark, MoreHoriz, OpenInNew, Refresh } from "@mui/icons-material";
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
import { useCallback, useEffect, useState } from "react";
import useAbortController from "../../hooks/useAbortController";
import { useArticles } from "../../hooks/useArticles";
import { useBookmarks } from "../../hooks/useBookmarks";
import useEntries from "../../hooks/useEntries";
import { useView } from "../../hooks/useView";
import { usePlatform } from "../../platform/Platform";
import { buildEntryHtml } from "../../text/buildEntryHtml";
import { markdownToHtml } from "../../text/markdownPipeline";
import { type Entry, type Item } from "../../types";
import cx from "../../utils/cx";
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
  const { view, themeMode } = useView();
  const [abort, abortControllerRef] = useAbortController();
  const [html, setHtml] = useState<string | null>(null);
  const [menuAnchor, setMenuAnchor] = useState<HTMLElement | null>(null);

  const { bookmarkMap, addBookmark, removeBookmark } =
    useBookmarks(abortControllerRef);

  const { markEntryRead, markEntryUnread } = useEntries(abortControllerRef);

  const {
    loading,
    readArticle,
    loadArticle,
    findArticleByUrl,
    loadArticleByUrl,
  } = useArticles(abortControllerRef);

  const requestArticle = useCallback(
    async (entry: Entry) => {
      setHtml(null);

      if (view === "full") {
        // readArticle saves the item, hence marking it read.
        const article = await readArticle(entry);
        const markdown = article?.content || article?.summary;

        if (markdown) {
          const body = await markdownToHtml(markdown);
          if (body) setHtml(buildEntryHtml(body, themeMode));
        }
      } else {
        const item = await markEntryRead(entry);
        if (item) onChangeEntry?.(item);

        if (entry.summary) {
          const body = await markdownToHtml(entry.summary);
          if (body) setHtml(buildEntryHtml(body, themeMode));
        }
      }
    },
    [view, readArticle, themeMode, markEntryRead, onChangeEntry],
  );

  const reloadArticle = useCallback(
    async (entry: Entry) => {
      setHtml(null);

      const article = await loadArticle(entry);

      if (article?.content) {
        const body = await markdownToHtml(article.content);
        if (body) setHtml(buildEntryHtml(body, themeMode));
      }
    },
    [themeMode, loadArticle],
  );

  const handleOpen = useCallback(() => {
    platform.openUrl(selectedEntry.url);
  }, [platform, selectedEntry]);

  const toggleBookmark = useCallback(async () => {
    if (bookmarkMap?.get(selectedEntry.id)) {
      await removeBookmark(selectedEntry);
    } else {
      await addBookmark(selectedEntry);
    }
  }, [addBookmark, bookmarkMap, removeBookmark, selectedEntry]);

  const handleLoadArticleByUrl = useCallback(
    async (url: string) => {
      setHtml(null);

      let article = await findArticleByUrl(url);

      if (!article) {
        article = await loadArticleByUrl(url);
      }

      if (article?.content) {
        const body = await markdownToHtml(article.content);
        if (body) setHtml(buildEntryHtml(body, themeMode));
      }

      if (article) onSelectEntry(article);
    },
    [findArticleByUrl, loadArticleByUrl, onSelectEntry, themeMode],
  );

  const handleMarkRead = useCallback(async () => {
    const item = await markEntryRead(selectedEntry);
    if (item) onChangeEntry?.(item);
    setMenuAnchor(null);
  }, [markEntryRead, onChangeEntry, selectedEntry]);

  const handleMarkUnread = useCallback(async () => {
    const item = await markEntryUnread(selectedEntry);
    if (item) onChangeEntry?.(item);
    setMenuAnchor(null);
  }, [markEntryUnread, onChangeEntry, selectedEntry]);

  const handleReload = useCallback(async () => {
    reloadArticle(selectedEntry);
    setMenuAnchor(null);
  }, [reloadArticle, selectedEntry]);

  useEffect(() => {
    abort();
    (async () => {
      requestArticle(selectedEntry);
    })();
  }, [requestArticle, abort, selectedEntry, view]);

  useEffect(() => {
    setLoading?.(loading);
  }, [loading, setLoading]);

  return (
    <>
      <div className={css.Header}>
        <div className={css.Title}>
          <Typography component="h1" variant="h5">
            {selectedEntry.title}
          </Typography>

          <ButtonGroup>
            <Tooltip
              title={
                bookmarkMap?.get(selectedEntry.id)
                  ? "Remove bookmark"
                  : "Add bookmark"
              }
            >
              <IconButton onClick={toggleBookmark}>
                <Bookmark
                  color={
                    bookmarkMap?.get(selectedEntry.id) ? "primary" : undefined
                  }
                />
              </IconButton>
            </Tooltip>

            <Tooltip title="Open in browser">
              <IconButton onClick={handleOpen}>
                <OpenInNew />
              </IconButton>
            </Tooltip>
          </ButtonGroup>
        </div>
      </div>

      <div className={css.Content}>
        <div className={css.Entry}>
          <div className={css.Byline}>
            <Typography variant="caption" color="text.secondary">
              {formatByline(selectedEntry)}
            </Typography>

            <Tooltip title="More actions">
              <IconButton
                size="small"
                onClick={(e) => setMenuAnchor(e.currentTarget)}
              >
                <MoreHoriz fontSize="small" />
              </IconButton>
            </Tooltip>

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

            {selectedEntry?.summary ? (
              <div className={css.Advisory}>
                <Typography variant="caption">
                  Here's a summary while loading your article...
                </Typography>
              </div>
            ) : (
              <div className={css.Advisory}>
                <Typography variant="caption">
                  Loading your article...
                </Typography>
              </div>
            )}
          </Collapse>

          {html && (
            <Reader
              className={cx(className)}
              html={html}
              onClickUrl={handleLoadArticleByUrl}
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
