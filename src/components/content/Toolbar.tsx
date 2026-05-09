import {
  Bookmark,
  Circle,
  CircleOutlined,
  OpenInNew,
  Refresh,
} from "@mui/icons-material";
import { IconButton, Tooltip } from "@mui/material";
import { useCallback, useEffect, useState } from "react";
import useAbortController from "../../hooks/useAbortController";
import { useBookmarks } from "../../hooks/useBookmarks";
import useEntries from "../../hooks/useEntries";
import { useEvents } from "../../hooks/useEvents";
import useRouter from "../../hooks/useRouter";
import { usePlatform } from "../../platform/Platform";
import { type Entry } from "../../types";
import { AddressBar } from "../AddressBar";
import { Search } from "../Search";
import css from "./Toolbar.module.css";

interface ToolbarProps {
  reload: (() => void) | null;
}

export function Toolbar({ reload }: ToolbarProps) {
  const [, abortControllerRef] = useAbortController();
  const { addBookmark, removeBookmark } = useBookmarks(abortControllerRef);
  const { markEntryRead, markEntryUnread } = useEntries(abortControllerRef);
  const platform = usePlatform();
  const {
    state: { url, entryId },
  } = useRouter();
  const { onBookmarkChanged, onUnreadChanged } = useEvents();

  const [latestEntry, setLatestEntry] = useState<Entry | null>(null);
  const entry = latestEntry?.id === entryId ? latestEntry : null;

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

  useEffect(() => onBookmarkChanged(setLatestEntry), [onBookmarkChanged]);
  useEffect(() => onUnreadChanged(setLatestEntry), [onUnreadChanged]);

  const toggleBookmark = useCallback(async () => {
    if (!entry) return;
    if (entry.isBookmarked) await removeBookmark(entry);
    else await addBookmark(entry);
  }, [entry, addBookmark, removeBookmark]);

  const handleOpenUrl = useCallback(() => {
    if (!url) return;
    void platform.openUrl(url);
  }, [url, platform]);

  const toggleRead = useCallback(async () => {
    if (!entry) return;
    if (entry.isUnread) await markEntryRead(entry.id);
    else await markEntryUnread(entry.id);
  }, [entry, markEntryRead, markEntryUnread]);

  return (
    <div className={css.Toolbar}>
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

      <Tooltip title={entry?.isUnread ? "Mark as read" : "Mark as unread"}>
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

      <Tooltip title={entry?.isBookmarked ? "Remove bookmark" : "Add bookmark"}>
        <span>
          <IconButton size="small" onClick={toggleBookmark} disabled={!entry}>
            <Bookmark
              fontSize="small"
              color={entry?.isBookmarked ? "primary" : undefined}
            />
          </IconButton>
        </span>
      </Tooltip>

      <Tooltip title="Open in browser">
        <span>
          <IconButton size="small" onClick={handleOpenUrl} disabled={!url}>
            <OpenInNew fontSize="small" />
          </IconButton>
        </span>
      </Tooltip>

      <Search />
    </div>
  );
}
