import {
  ArrowBack,
  ArrowForward,
  Bookmark,
  Circle,
  CircleOutlined,
  Language,
  OpenInNew,
  QrCode2,
  Refresh,
  Search,
} from "@mui/icons-material";
import {
  Collapse,
  IconButton,
  LinearProgress,
  Popover,
  Tooltip,
} from "@mui/material";
import { useCallback, useEffect, useRef, useState } from "react";
import QRCode from "react-qr-code";
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
import { TopicMenuButton } from "./TopicMenuButton";

interface ToolbarProps {
  reload: (() => void) | null;
}

export function Toolbar({ reload }: ToolbarProps) {
  const platform = usePlatform();
  const {
    goBack: back,
    goForward: forward,
    canGoBack,
    canGoForward,
    selectUrl,
    state,
    invokeSearch,
    clearSearch,
    search,
  } = useRouter();
  const isSearchOpen = typeof search === "string";
  const [, abortControllerRef] = useAbortController();
  const { addBookmark, removeBookmark } = useBookmarks(abortControllerRef);
  const { markEntryRead, markEntryUnread } = useEntries(abortControllerRef);
  const { onBookmarkChanged, onUnreadChanged } = useEvents();
  const { loadArticleByUrl, loading } = useArticles(abortControllerRef);

  const { url, entryId } = state;
  const [latestEntry, setLatestEntry] = useState<Entry | null>(null);

  // Reset on navigation so button states stay cleared until the current
  // entry is loaded; a lingering latestEntry from a previous visit to the
  // same entry may hold stale flags.
  const [lastEntryId, setLastEntryId] = useState(entryId);
  if (lastEntryId !== entryId) {
    setLastEntryId(entryId);
    setLatestEntry(null);
  }

  const entry = latestEntry?.id === entryId ? latestEntry : null;
  const [favicon, setFavicon] = useState<string | null>(null);
  const qrButtonRef = useRef<HTMLButtonElement>(null);
  const [qrOpen, setQrOpen] = useState(false);

  const handleSearchToggle = useCallback(() => {
    if (isSearchOpen) clearSearch();
    else invokeSearch("");
  }, [isSearchOpen, invokeSearch, clearSearch]);

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

  const handleNavigate = useCallback(
    (inputUrl: string) => {
      if (!/^https?:\/\//i.test(inputUrl)) inputUrl = `https://${inputUrl}`;
      selectUrl(inputUrl);
    },
    [selectUrl]
  );

  const handleShowOriginalSite = useCallback(async () => {
    if (!url) return;
    await loadArticleByUrl(url, true);
  }, [url, loadArticleByUrl]);

  const handleOpenInBrowser = useCallback(() => {
    if (!url) return;
    platform.openUrl(url);
  }, [platform, url]);

  const handleQrOpen = useCallback(() => setQrOpen(true), []);
  const handleQrClose = useCallback(() => setQrOpen(false), []);

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
      .then((article) => {
        if (controller.signal.aborted) return;
        setLatestEntry((prev) => (prev?.id === entryId ? prev : article));
      })
      .catch(() => {});
    return () => controller.abort();
  }, [entryId, platform]);

  useEffect(() => {
    (async () => {
      setFavicon(null);
      if (!url) return;
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

  useEffect(
    () =>
      onBookmarkChanged((entry) => {
        if (entry.id === entryId) setLatestEntry(entry);
      }),
    [onBookmarkChanged, entryId]
  );
  useEffect(
    () =>
      onUnreadChanged((entry) => {
        if (entry.id === entryId) setLatestEntry(entry);
      }),
    [onUnreadChanged, entryId]
  );

  return (
    <div className={css.Toolbar}>
      <Tooltip title="Back">
        <IconButton
          size="small"
          onClick={back}
          disabled={!canGoBack}
          aria-label="Back"
        >
          <ArrowBack fontSize="small" />
        </IconButton>
      </Tooltip>
      <Tooltip title="Forward">
        <IconButton
          size="small"
          onClick={forward}
          disabled={!canGoForward}
          aria-label="Forward"
        >
          <ArrowForward fontSize="small" />
        </IconButton>
      </Tooltip>
      <div className={css.AddressBarContainer}>
        <div>
          <div className={css.AddressBar}>
            <Tooltip title="Show Original Site">
              <IconButton
                size="small"
                onClick={handleShowOriginalSite}
                disabled={!entry}
                aria-label="Show Original Site"
              >
                {favicon ? (
                  <div className={css.Favicon}>
                    <img
                      src={favicon}
                      width={16}
                      height={16}
                      alt="Show original site"
                      style={{ margin: "0 2px" }}
                    />
                  </div>
                ) : (
                  <Language fontSize="small" />
                )}
              </IconButton>
            </Tooltip>

            <AddressBar url={url ?? null} onNavigate={handleNavigate} />

            <Tooltip title="Refresh">
              <IconButton
                size="small"
                onClick={reload ?? undefined}
                disabled={!reload}
                aria-label="Refresh"
                className={css.RevealOnHover}
              >
                <Refresh fontSize="small" />
              </IconButton>
            </Tooltip>

            <Tooltip title="Open in Browser">
              <IconButton
                size="small"
                onClick={handleOpenInBrowser}
                disabled={!url}
                aria-label="Open in Browser"
                className={css.RevealOnHover}
              >
                <OpenInNew fontSize="small" />
              </IconButton>
            </Tooltip>

            <Tooltip title="Show QR Code">
              <IconButton
                ref={qrButtonRef}
                size="small"
                onClick={handleQrOpen}
                disabled={!url}
                aria-label="Show QR Code"
                className={css.RevealOnHover}
              >
                <QrCode2 fontSize="small" />
              </IconButton>
            </Tooltip>

            <Tooltip
              title={entry?.isUnread ? "Mark as read" : "Mark as unread"}
            >
              <IconButton
                size="small"
                onClick={toggleRead}
                disabled={!entry}
                aria-label={entry?.isUnread ? "Mark as read" : "Mark as unread"}
                className={css.RevealOnHover}
              >
                {entry?.isUnread ? (
                  <Circle fontSize="small" color="primary" />
                ) : (
                  <CircleOutlined fontSize="small" />
                )}
              </IconButton>
            </Tooltip>

            <Tooltip
              title={entry?.isBookmarked ? "Remove bookmark" : "Add bookmark"}
            >
              <IconButton
                size="small"
                onClick={toggleBookmark}
                disabled={!entry}
                aria-label={
                  entry?.isBookmarked ? "Remove bookmark" : "Add bookmark"
                }
              >
                <Bookmark
                  fontSize="small"
                  color={entry?.isBookmarked ? "primary" : undefined}
                />
              </IconButton>
            </Tooltip>

            <TopicMenuButton item={entry} disabled={!entry} />
          </div>

          <Popover
            open={qrOpen}
            anchorEl={() => qrButtonRef.current}
            onClose={handleQrClose}
            anchorOrigin={{ vertical: "bottom", horizontal: "center" }}
            transformOrigin={{ vertical: "top", horizontal: "center" }}
          >
            {url && (
              <div className={css.QrPopover}>
                <QRCode value={url} size={160} />
              </div>
            )}
          </Popover>
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
