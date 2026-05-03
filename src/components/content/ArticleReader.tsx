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
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
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
  entryId: Entry["id"];
}

export default function ArticleReader({ className, entryId }: ArticleReaderProps) {
  const navigate = useNavigate();
  const { collectionId } = useParams();
  const [searchParams] = useSearchParams();

  const platform = usePlatform();

  const {
    readerView,
    themeMode,
    readerFontFamily,
    readerFontSize,
    readerLineHeight,
  } = useAppearance();

  const [abort, abortControllerRef] = useAbortController();
  const [selectedEntry, setSelectedEntry] = useState<Entry | null>(null);
  const [bodyHtml, setBodyHtml] = useState<string | null>(null);
  const [menuAnchor, setMenuAnchor] = useState<HTMLElement | null>(null);

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

  const { addBookmark, removeBookmark } = useBookmarks(abortControllerRef);
  const { markEntryRead, markEntryUnread } = useEntries(abortControllerRef);
  const { setFindEnabled } = useEvents();

  useEffect(() => {
    setFindEnabled(!!html);
    return () => { setFindEnabled(false); };
  }, [html, setFindEnabled]);

  const {
    loading,
    readArticle,
    loadArticle,
    findArticleByUrl,
    loadArticleByUrl,
  } = useArticles(abortControllerRef);

  const navigateToItem = useCallback(
    (item: Item) => {
      const params = new URLSearchParams(searchParams);
      navigate(`/${collectionId}/${item.id}?${params.toString()}`);
    },
    [navigate, collectionId, searchParams],
  );

  const requestArticle = useCallback(
    async (entryId: Entry["id"]) => {
      setBodyHtml(null);

      const article = await readArticle(entryId);

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
      setBodyHtml(null);
      const article = await loadArticle(entry);
      if (article?.content) {
        const body = await markdownToHtml(article.content);
        if (body) setBodyHtml(body);
      }
    },
    [loadArticle]
  );

  const handleOpenUrl = useCallback(() => {
    if (!selectedEntry?.url) return;
    platform.openUrl(selectedEntry.url);
  }, [platform, selectedEntry]);

  const toggleBookmark = useCallback(async () => {
    if (!selectedEntry) return;
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
      if (!article) article = await loadArticleByUrl(url);

      if (article?.content) {
        const body = await markdownToHtml(article.content);
        if (body) setBodyHtml(body);
      }

      if (article) navigateToItem(article);
    },
    [findArticleByUrl, loadArticleByUrl, navigateToItem]
  );

  const handleMarkRead = useCallback(async () => {
    if (!entryId) return;
    await markEntryRead(entryId);
    setMenuAnchor(null);
  }, [markEntryRead, entryId]);

  const handleMarkUnread = useCallback(async () => {
    if (!entryId) return;
    await markEntryUnread(entryId);
    setMenuAnchor(null);
  }, [markEntryUnread, entryId]);

  const handleReload = useCallback(async () => {
    if (!selectedEntry) return;
    reloadArticle(selectedEntry);
    setMenuAnchor(null);
  }, [reloadArticle, selectedEntry]);

  useEffect(() => {
    if (!entryId) return;
    abort();
    (async () => { requestArticle(entryId); })();
  }, [requestArticle, abort, entryId, readerView]);

  return (
    <>
      <div className={css.Header}>
        <div className={css.Title}>
          <Typography
            component="h1"
            variant="h5"
            sx={{ fontFamily: FONT_FAMILY_CSS[readerFontFamily] }}
          >
            {selectedEntry?.title}
          </Typography>
        </div>
      </div>

      <div className={css.Content}>
        <div className={css.Entry}>
          <div className={css.Byline}>
            <Typography variant="caption" color="text.secondary">
              {selectedEntry && formatByline(selectedEntry)}
            </Typography>

            <ButtonGroup>
              <Tooltip
                title={selectedEntry?.isBookmarked ? "Remove bookmark" : "Add bookmark"}
              >
                <IconButton size="small" onClick={toggleBookmark}>
                  <Bookmark color={selectedEntry?.isBookmarked ? "primary" : undefined} />
                </IconButton>
              </Tooltip>

              <Tooltip title="Open in browser">
                <IconButton size="small" onClick={handleOpenUrl}>
                  <OpenInNew />
                </IconButton>
              </Tooltip>

              <Tooltip title="More actions">
                <IconButton size="small" onClick={(e) => setMenuAnchor(e.currentTarget)}>
                  <MoreVert fontSize="small" />
                </IconButton>
              </Tooltip>
            </ButtonGroup>

            <Menu
              anchorEl={menuAnchor}
              open={Boolean(menuAnchor)}
              onClose={() => setMenuAnchor(null)}
            >
              {selectedEntry?.isUnread ? (
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

          {selectedEntry && html && (
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

        {selectedEntry && (
          <aside>
            <RelatedItemList itemId={selectedEntry.id} />
          </aside>
        )}
      </div>
    </>
  );
}
