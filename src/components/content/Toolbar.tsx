import {
  InfoOutlined,
  ArrowBack,
  ArrowForward,
  Bookmark,
  Circle,
  CircleOutlined,
  Language,
  OpenInNew,
  PushPin,
  PushPinOutlined,
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
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type RefObject,
} from "react";
import QRCode from "react-qr-code";
import useAbortController from "../../hooks/useAbortController";
import { useArticles } from "../../hooks/useArticles";
import { useBookmarks } from "../../hooks/useBookmarks";
import useEntries from "../../hooks/useEntries";
import { subscribe, useEvents } from "../../hooks/useEvents";
import { useLibraryReadOnly } from "../../hooks/useLibraryReadOnly";
import { useErrorSink } from "../../hooks/useErrorSink";
import FaviconLoadError from "../../utils/FaviconLoadError";
import { isLocalFaviconReference } from "../../utils/isLocalFaviconReference";
import useRouter from "../../hooks/useRouter";
import { usePins } from "../../providers/PinsContext";
import { usePlatform } from "../../platform/Platform";
import { useEmbeddingStartupState } from "../../hooks/useEmbeddingStartupState";
import { isArticle, type Entry } from "../../types";
import { AddressBar } from "../AddressBar";
import css from "./Toolbar.module.css";
import { TopicMenuButton } from "./TopicMenuButton";

interface ToolbarProps {
  reload: (() => void) | null;
  reloading?: boolean;
  inspectorOpener?: RefObject<HTMLButtonElement | null>;
  onOpenInspector?: () => void;
  inspectorOpen?: boolean;
}

export function Toolbar({
  reload,
  reloading = false,
  inspectorOpener,
  onOpenInspector,
  inspectorOpen,
}: ToolbarProps) {
  const platform = usePlatform();
  const { pins, setPin } = usePins();
  const isLibraryReadOnly = useLibraryReadOnly();
  const [pinning, setPinning] = useState(false);
  const embeddingStartupState = useEmbeddingStartupState();
  const { pushError } = useErrorSink();
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
  const { onBookmarkChanged, onUnreadChanged, onFaviconChanged } = useEvents();
  const { loadArticleByUrl, loading } = useArticles(abortControllerRef);

  const { url, entryId } = state;
  const [latestEntry, setLatestEntry] = useState<Entry | null>(null);
  const [favicon, setFavicon] = useState<{
    reference: string;
    data: string | null;
  } | null>(null);

  // Reset on navigation so button states stay cleared until the current
  // entry is loaded; a lingering latestEntry from a previous visit to the
  // same entry may hold stale flags.
  const [lastEntryId, setLastEntryId] = useState(entryId);
  if (lastEntryId !== entryId) {
    setLastEntryId(entryId);
    setLatestEntry(null);
    setFavicon(null);
  }

  const entry = useMemo(
    () =>
      latestEntry && latestEntry.id === entryId
        ? {
            ...latestEntry,
            isPinned: pins.some(
              (pin) => pin.type === "entry" && pin.id === entryId
            ),
          }
        : null,
    [latestEntry, entryId, pins]
  );

  // Only persisted local references may supply an icon. Never render a stored
  // remote URL directly: that would fetch from the privileged main window.
  const storedFavicon = isArticle(entry) ? entry.favicon : undefined;
  const isLocalFaviconRef = isLocalFaviconReference(storedFavicon);
  const displayFavicon =
    isLocalFaviconRef && favicon && favicon.reference === storedFavicon
      ? favicon.data
      : null;

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

  const togglePin = useCallback(async () => {
    if (!entry || pinning) return;
    setPinning(true);
    try {
      await setPin({ type: "entry", id: entry.id }, !entry.isPinned);
    } catch (error) {
      pushError(error);
    } finally {
      setPinning(false);
    }
  }, [entry, pinning, setPin, pushError]);

  useEffect(
    () =>
      subscribe(() =>
        platform.onEntryPinChanged((updated) => {
          if (updated.id === entryId) setLatestEntry(updated);
        })
      ),
    [entryId, platform]
  );

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

  // The favicon is fetched inside the article loader at ingestion and stored
  // in the library, so displaying it makes no network request from this
  // privileged window. The bytes come back over IPC.
  useEffect(() => {
    if (!entry || !storedFavicon) return;
    const reportFailure = () =>
      pushError(new FaviconLoadError(entry.url, true));
    if (!isLocalFaviconRef) {
      reportFailure();
      return;
    }

    const controller = new AbortController();
    platform
      .readEntryImages(controller.signal, entry.id, [storedFavicon])
      .then(([dataUri]) => {
        if (controller.signal.aborted) return;
        setFavicon({ reference: storedFavicon, data: dataUri ?? null });
        if (!dataUri) reportFailure();
      })
      .catch(() => {
        if (!controller.signal.aborted) reportFailure();
      });
    return () => controller.abort();
  }, [entry, storedFavicon, isLocalFaviconRef, platform, pushError]);

  useEffect(
    () =>
      onBookmarkChanged((entry) => {
        if (entry.id === entryId)
          setLatestEntry((current) => ({
            ...entry,
            isPinned: current?.isPinned ?? entry.isPinned,
          }));
      }),
    [onBookmarkChanged, entryId]
  );
  useEffect(
    () =>
      onFaviconChanged((article) => {
        if (article.id === entryId) setLatestEntry(article);
      }),
    [onFaviconChanged, entryId]
  );
  useEffect(
    () =>
      onUnreadChanged((entry) => {
        if (entry.id === entryId)
          setLatestEntry((current) => ({
            ...entry,
            isPinned: current?.isPinned ?? entry.isPinned,
          }));
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
                {displayFavicon ? (
                  <div className={css.Favicon}>
                    <img
                      src={displayFavicon}
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

            <Tooltip title={reloading ? "Reload in progress..." : "Refresh"}>
              <span>
                <IconButton
                  size="small"
                  onClick={reloading ? undefined : (reload ?? undefined)}
                  disabled={!reload}
                  title={reloading ? "Reload in progress..." : "Refresh"}
                  aria-label={reloading ? "Reload in progress..." : "Refresh"}
                >
                  <Refresh
                    fontSize="small"
                    className={reloading ? css.ReloadIconBusy : css.ReloadIcon}
                  />
                </IconButton>
              </span>
            </Tooltip>
            <span role="status" className={css.ScreenReaderOnly}>
              {reloading ? "Reload in progress..." : ""}
            </span>

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

            <Tooltip
              title={
                entry?.isPinned ? "Remove from Shortcuts" : "Add to Shortcuts"
              }
            >
              <span>
                <IconButton
                  size="small"
                  onClick={togglePin}
                  disabled={!entry || isLibraryReadOnly || pinning}
                  aria-label={
                    entry?.isPinned
                      ? "Remove from Shortcuts"
                      : "Add to Shortcuts"
                  }
                  aria-pressed={!!entry?.isPinned}
                >
                  {entry?.isPinned ? (
                    <PushPin fontSize="small" color="primary" />
                  ) : (
                    <PushPinOutlined fontSize="small" />
                  )}
                </IconButton>
              </span>
            </Tooltip>
            <TopicMenuButton item={entry} disabled={!entry} />
            {onOpenInspector && (
              <Tooltip title="Open Inspector">
                <IconButton
                  size="small"
                  ref={inspectorOpener}
                  aria-label="Open Inspector"
                  aria-haspopup="dialog"
                  aria-expanded={!!inspectorOpen}
                  onClick={onOpenInspector}
                >
                  <InfoOutlined fontSize="small" />
                </IconButton>
              </Tooltip>
            )}
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
        <Tooltip
          title={
            embeddingStartupState === "available"
              ? "Show Search"
              : "Search unavailable"
          }
        >
          <span>
            <IconButton
              size="small"
              onClick={handleSearchToggle}
              aria-label="Show Search"
              disabled={embeddingStartupState !== "available"}
            >
              <Search fontSize="small" />
            </IconButton>
          </span>
        </Tooltip>
      </Collapse>
    </div>
  );
}
