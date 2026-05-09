import { Bookmark, Circle, CircleOutlined, OpenInNew, Refresh } from "@mui/icons-material";
import { IconButton, Tooltip } from "@mui/material";
import { useCallback } from "react";
import useAbortController from "../../hooks/useAbortController";
import { useBookmarks } from "../../hooks/useBookmarks";
import useEntries from "../../hooks/useEntries";
import { usePlatform } from "../../platform/Platform";
import { type Article } from "../../types";
import { AddressBar } from "../AddressBar";
import { Search } from "../Search";
import css from "./Toolbar.module.css";

interface ToolbarProps {
  entry: Article | null;
  reload: (() => void) | null;
}

export function Toolbar({ entry, reload }: ToolbarProps) {
  const [, abortControllerRef] = useAbortController();
  const { addBookmark, removeBookmark } = useBookmarks(abortControllerRef);
  const { markEntryRead, markEntryUnread } = useEntries(abortControllerRef);
  const platform = usePlatform();

  const toggleBookmark = useCallback(async () => {
    if (!entry) return;
    if (entry.isBookmarked) await removeBookmark(entry);
    else await addBookmark(entry);
  }, [entry, addBookmark, removeBookmark]);

  const handleOpenUrl = useCallback(() => {
    if (!entry?.url) return;
    void platform.openUrl(entry.url);
  }, [entry, platform]);

  const toggleRead = useCallback(async () => {
    if (!entry) return;
    if (entry.isUnread) await markEntryRead(entry.id);
    else await markEntryUnread(entry.id);
  }, [entry, markEntryRead, markEntryUnread]);

  return (
    <div className={css.Toolbar}>
      <AddressBar url={entry?.url ?? null} />

      <Tooltip title="Refresh">
        <span>
          <IconButton size="small" onClick={reload ?? undefined} disabled={!reload}>
            <Refresh fontSize="small" />
          </IconButton>
        </span>
      </Tooltip>

      <Tooltip title={entry?.isUnread ? "Mark as read" : "Mark as unread"}>
        <span>
          <IconButton size="small" onClick={toggleRead} disabled={!entry}>
            {entry?.isUnread
              ? <Circle fontSize="small" color="primary" />
              : <CircleOutlined fontSize="small" />}
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
          <IconButton size="small" onClick={handleOpenUrl} disabled={!entry}>
            <OpenInNew fontSize="small" />
          </IconButton>
        </span>
      </Tooltip>

      <Search />
    </div>
  );
}
