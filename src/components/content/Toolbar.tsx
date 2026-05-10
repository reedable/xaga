import {
  Bookmark,
  Circle,
  CircleOutlined,
  Language,
  Refresh,
  Search,
} from "@mui/icons-material";
import { Collapse, IconButton, LinearProgress, Tooltip } from "@mui/material";
import { useCallback, useEffect, useState } from "react";
import useAbortController from "../../hooks/useAbortController";
import { useArticles } from "../../hooks/useArticles";
import { useBookmarks } from "../../hooks/useBookmarks";
import useEntries from "../../hooks/useEntries";
import { useEvents } from "../../hooks/useEvents";
import useRouter from "../../hooks/useRouter";
import { usePlatform } from "../../platform/Platform";
import { isArticle, type Entry } from "../../types";
import { AddressBar } from "../AddressBar";
import css from "./Toolbar.module.css";

interface ToolbarProps {
  reload: (() => void) | null;
}

export function Toolbar({ reload }: ToolbarProps) {
  const platform = usePlatform();
  const { state, search, clearSearch, searchParams } = useRouter();
  const isSearchOpen = searchParams.get("search") !== null;

  const [, abortControllerRef] = useAbortController();
  const { addBookmark, removeBookmark } = useBookmarks(abortControllerRef);
  const { markEntryRead, markEntryUnread } = useEntries(abortControllerRef);
  const { onBookmarkChanged, onUnreadChanged } = useEvents();
  const { loading } = useArticles(abortControllerRef);

  const { url, entryId } = state;
  const [latestEntry, setLatestEntry] = useState<Entry | null>(null);
  const entry = latestEntry?.id === entryId ? latestEntry : null;
  const [favicon, setFavicon] = useState<string | null>(null);

  const handleSearchToggle = useCallback(() => {
    if (isSearchOpen) clearSearch();
    else search("");
  }, [isSearchOpen, search, clearSearch]);

  const toggleBookmark = useCallback(async () => {
    if (!entry) return;
    if (entry.isBookmarked) await removeBookmark(entry);
    else await addBookmark(entry);
  }, [entry, addBookmark, removeBookmark]);

  const toggleRead = useCallback(async () => {
    if (!entry) return;
    if (entry.isUnread) await markEntryRead(entry.id);
    else await markEntryUnread(entry.id);
  }, [entry, markEntryRead, markEntryUnread]);

  const handleOpenUrl = useCallback(() => {
    if (!url) return;
    void platform.openUrl(url);
  }, [url, platform]);

  const getFavicon = useCallback(async (url?: string) => {
    if (!url) return null;

    try {
      const img = new Image();

      await new Promise<void>((resolve) => {
        img.onload = () => resolve();
        img.src = url;
      });

      return url;
    } catch {
      return null;
    }
  }, []);

  useEffect(() => {
    if (!entryId) return;
    const controller = new AbortController();
    platform
      .readArticle(controller.signal, entryId)
      .then((article) =>
        setLatestEntry((prev) => (prev?.id === entryId ? prev : article))
      )
      .catch(() => {});
    return () => controller.abort();
  }, [entryId, platform]);

  useEffect(() => {
    (async () => {
      setFavicon(null);
      const faviconUrl = `${new URL(url).origin}/favicon.ico`;
      setFavicon(await getFavicon(faviconUrl));
    })();
  }, [getFavicon, url]);

  useEffect(() => {
    (async () => {
      if (isArticle(entry) && entry.favicon) {
        setFavicon(await getFavicon(entry.favicon));
      }
    })();
  }, [getFavicon, entry]);

  useEffect(() => onBookmarkChanged(setLatestEntry), [onBookmarkChanged]);
  useEffect(() => onUnreadChanged(setLatestEntry), [onUnreadChanged]);

  return (
    <div className={css.Toolbar}>
      <div className={css.AddressBarContainer}>
        <div>
          <div className={css.AddressBar}>
            <Tooltip title="Open in browser">
              <span>
                <IconButton
                  size="small"
                  onClick={handleOpenUrl}
                  disabled={!url}
                >
                  {favicon ? (
                    <div className={css.Favicon}>
                      <img
                        src={favicon}
                        width={16}
                        height={16}
                        alt="Open in browser"
                        style={{ margin: "0 2px" }}
                      />
                    </div>
                  ) : (
                    <Language fontSize="small" />
                  )}
                </IconButton>
              </span>
            </Tooltip>

            <AddressBar url={entry?.url ?? url ?? null} />

            <Tooltip title="Refresh">
              <span>
                <IconButton
                  size="small"
                  onClick={reload ?? undefined}
                  disabled={!reload}
                >
                  <Refresh fontSize="small" />
                </IconButton>
              </span>
            </Tooltip>

            <Tooltip
              title={entry?.isUnread ? "Mark as read" : "Mark as unread"}
            >
              <span>
                <IconButton size="small" onClick={toggleRead} disabled={!entry}>
                  {entry?.isUnread ? (
                    <Circle fontSize="small" color="primary" />
                  ) : (
                    <CircleOutlined fontSize="small" />
                  )}
                </IconButton>
              </span>
            </Tooltip>

            <Tooltip
              title={entry?.isBookmarked ? "Remove bookmark" : "Add bookmark"}
            >
              <span>
                <IconButton
                  size="small"
                  onClick={toggleBookmark}
                  disabled={!entry}
                >
                  <Bookmark
                    fontSize="small"
                    color={entry?.isBookmarked ? "primary" : undefined}
                  />
                </IconButton>
              </span>
            </Tooltip>
          </div>
        </div>

        <Collapse in={loading} unmountOnExit>
          <LinearProgress />
        </Collapse>
      </div>

      <Collapse in={!isSearchOpen} orientation="horizontal">
        <Tooltip title="Show Search">
          <IconButton
            size="small"
            onClick={handleSearchToggle}
            aria-label="Show Search"
          >
            <Search fontSize="small" />
          </IconButton>
        </Tooltip>
      </Collapse>
    </div>
  );
}
