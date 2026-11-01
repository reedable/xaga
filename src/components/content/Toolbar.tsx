import {
  ArrowBack,
  ArrowForward,
  Bookmark,
  Circle,
  CircleOutlined,
  InfoOutlined,
  IosShare,
  Language,
  OpenInNew,
  PushPin,
  PushPinOutlined,
  QrCode2,
  Refresh,
  Search,
} from "@mui/icons-material";
import { Collapse, Divider, IconButton, LinearProgress, Menu, MenuItem, Popover, Tooltip } from "@mui/material";
import { useCallback, useEffect, useMemo, useRef, useState, type RefObject } from "react";
import QRCode from "react-qr-code";
import colorCss from "../../colors.module.css";
import useAbortController from "../../hooks/useAbortController";
import { useArticles } from "../../hooks/useArticles";
import { useBookmarks } from "../../hooks/useBookmarks";
import { useEmbeddingStartupState } from "../../hooks/useEmbeddingStartupState";
import useEntries from "../../hooks/useEntries";
import { useErrorSink } from "../../hooks/useErrorSink";
import { subscribe, useEvents } from "../../hooks/useEvents";
import { useLibraryReadOnly } from "../../hooks/useLibraryReadOnly";
import useRouter from "../../hooks/useRouter";
import { usePlatform } from "../../platform/Platform";
import { usePins } from "../../providers/PinsContext";
import { isArticle, type Entry } from "../../types";
import FaviconLoadError from "../../utils/FaviconLoadError";
import { isLocalFaviconReference } from "../../utils/isLocalFaviconReference";
import { AddressBar } from "../AddressBar";
import { ColorSwatches } from "./ColorSwatches";
import css from "./Toolbar.module.css";

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
  const {
    addBookmark,
    removeBookmark,
    loading: marking,
    error: markingError,
  } = useBookmarks(abortControllerRef);
  const [bookmarkAnchor, setBookmarkAnchor] = useState<HTMLElement | null>(null);
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
            isPinned: pins.some((pin) => pin.type === "entry" && pin.id === entryId),
          }
        : null,
    [latestEntry, entryId, pins],
  );

  // Only persisted local references may supply an icon. Never render a stored
  // remote URL directly: that would fetch from the privileged main window.
  const storedFavicon = isArticle(entry) ? entry.favicon : undefined;
  const isLocalFaviconRef = isLocalFaviconReference(storedFavicon);
  const displayFavicon =
    isLocalFaviconRef && favicon && favicon.reference === storedFavicon ? favicon.data : null;

  const shareButtonRef = useRef<HTMLButtonElement>(null);
  const [shareAnchor, setShareAnchor] = useState<HTMLElement | null>(null);
  const [qrOpen, setQrOpen] = useState(false);

  const handleSearchToggle = useCallback(() => {
    if (isSearchOpen) clearSearch();
    else invokeSearch("");
  }, [isSearchOpen, invokeSearch, clearSearch]);

  const changeBookmark = useCallback(
    async (color: string | null) => {
      if (!entry || marking || isLibraryReadOnly) return;
      const updated =
        color === null ? await removeBookmark(entry) : await addBookmark(entry, color);
      if (updated) {
        setLatestEntry(updated);
        setBookmarkAnchor(null);
      }
    },
    [entry, marking, isLibraryReadOnly, addBookmark, removeBookmark],
  );
  useEffect(() => {
    if (markingError) pushError(markingError);
  }, [markingError, pushError]);
  useEffect(() => {
    setBookmarkAnchor(null);
  }, [entryId]);
  useEffect(() => {
    const close = () => setBookmarkAnchor(null);
    window.addEventListener("blur", close);
    return () => window.removeEventListener("blur", close);
  }, []);

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
        }),
      ),
    [entryId, platform],
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
    [selectUrl],
  );

  const handleShowOriginalSite = useCallback(async () => {
    if (!url) return;
    await loadArticleByUrl(url, true);
  }, [url, loadArticleByUrl]);

  const handleOpenInBrowser = useCallback(() => {
    if (!url) return;
    setShareAnchor(null);
    platform.openUrl(url);
  }, [platform, url]);

  const handleQrOpen = useCallback(() => {
    setShareAnchor(null);
    setQrOpen(true);
  }, []);
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
    const reportFailure = () => pushError(new FaviconLoadError(entry.url, true));
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
    [onBookmarkChanged, entryId],
  );
  useEffect(
    () =>
      onFaviconChanged((article) => {
        if (article.id === entryId) setLatestEntry(article);
      }),
    [onFaviconChanged, entryId],
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
    [onUnreadChanged, entryId],
  );

  return (
    <div className={css.Toolbar}>
      <Tooltip title="Back">
        <IconButton size="small" onClick={back} disabled={!canGoBack} aria-label="Back">
          <ArrowBack fontSize="small" />
        </IconButton>
      </Tooltip>

      <Tooltip title="Forward">
        <IconButton size="small" onClick={forward} disabled={!canGoForward} aria-label="Forward">
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

            <Tooltip title="Share">
              <IconButton
                ref={shareButtonRef}
                size="small"
                onClick={(event) => setShareAnchor(event.currentTarget)}
                disabled={!url}
                aria-label="Share"
                aria-haspopup="menu"
                aria-expanded={!!shareAnchor}
                aria-controls={shareAnchor ? "toolbar-share-menu" : undefined}
              >
                <IosShare fontSize="small" />
              </IconButton>
            </Tooltip>
            <Menu
              id="toolbar-share-menu"
              anchorEl={shareAnchor}
              open={!!shareAnchor}
              onClose={() => setShareAnchor(null)}
              transitionDuration={0}
              slotProps={{ list: { "aria-label": "Share" } }}
            >
              <MenuItem onClick={handleOpenInBrowser}>
                <OpenInNew fontSize="small" sx={{ mr: 1 }} />
                Open in default browser
              </MenuItem>
              <MenuItem onClick={handleQrOpen}>
                <QrCode2 fontSize="small" sx={{ mr: 1 }} />
                Show QR code
              </MenuItem>
              <Divider />
              <MenuItem
                disabled={!entry || isLibraryReadOnly || pinning}
                onClick={() => {
                  setShareAnchor(null);
                  void togglePin();
                }}
              >
                {entry?.isPinned ? (
                  <PushPin fontSize="small" color="primary" sx={{ mr: 1 }} />
                ) : (
                  <PushPinOutlined fontSize="small" sx={{ mr: 1 }} />
                )}
                {entry?.isPinned ? "Remove from Shortcuts" : "Add to Shortcuts"}
              </MenuItem>
              <MenuItem
                disabled={!entry}
                onClick={() => {
                  setShareAnchor(null);
                  void toggleRead();
                }}
              >
                {entry?.isUnread ? (
                  <Circle fontSize="small" color="primary" sx={{ mr: 1 }} />
                ) : (
                  <CircleOutlined fontSize="small" sx={{ mr: 1 }} />
                )}
                {entry?.isUnread ? "Mark as read" : "Mark as unread"}
              </MenuItem>
            </Menu>

            <Tooltip title={entry?.color ? "Change bookmark color" : "Add bookmark"}>
              <span>
                <IconButton
                  size="small"
                  onClick={(event) => setBookmarkAnchor(event.currentTarget)}
                  disabled={!entry || isLibraryReadOnly || marking}
                  aria-label={entry?.color ? "Change bookmark color" : "Add bookmark"}
                  aria-haspopup="dialog"
                  aria-expanded={!!bookmarkAnchor}
                >
                  <Bookmark
                    fontSize="small"
                    className={entry?.color ? colorCss[entry.color] : undefined}
                    style={entry?.color ? { color: "var(--color-icon)" } : undefined}
                  />
                </IconButton>
              </span>
            </Tooltip>
            <Popover
              transitionDuration={0}
              open={!!bookmarkAnchor && !!entry}
              anchorEl={bookmarkAnchor}
              onClose={() => setBookmarkAnchor(null)}
              anchorOrigin={{ vertical: "bottom", horizontal: "center" }}
              transformOrigin={{ vertical: "top", horizontal: "center" }}
            >
              <div role="dialog" aria-label="Bookmark color" style={{ padding: 8 }}>
                <ColorSwatches
                  color={entry?.color ?? null}
                  onChange={changeBookmark}
                  disabled={marking || isLibraryReadOnly}
                  autoFocus
                />
              </div>
            </Popover>

            {onOpenInspector && (
              <Tooltip title="Show article details">
                <IconButton
                  size="small"
                  ref={inspectorOpener}
                  aria-label="Show article details"
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
            transitionDuration={0}
            open={qrOpen}
            anchorEl={() => shareButtonRef.current}
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
          title={embeddingStartupState === "available" ? "Show Search" : "Search unavailable"}
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
