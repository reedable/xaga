import {
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import EditFeedDialog from "../components/dialogs/EditFeedDialog";
import useAbortController from "../hooks/useAbortController";
import { useCollections } from "../hooks/useCollections";
import { useEvents } from "../hooks/useEvents";
import { useFeeds } from "../hooks/useFeeds";
import { useLocalStorage } from "../hooks/useLocalStorage";
import { usePlatform } from "../platform/Platform";
import { useNavigate } from "react-router-dom";
import type { Feed } from "../types";
import ManageFeedContext, {
  type ManageFeedContextType,
} from "./ManageFeedContext";

const sortByUnreadCount = (a: Feed, b: Feed) =>
  b.unreadCount - a.unreadCount || a.title.localeCompare(b.title);

interface ManageFeedProviderProps {
  children: ReactNode;
}

export function ManageFeedProvider({ children }: ManageFeedProviderProps) {
  const platform = usePlatform();
  const navigate = useNavigate();
  const [abort, abortControllerRef] = useAbortController();
  const { listFeeds } = useCollections(abortControllerRef);

  const {
    loadingSet,
    addFeed,
    renameFeed,
    deleteFeed,
    importFeeds,
    exportFeeds,
  } = useFeeds(abortControllerRef);

  const { onUnreadChanged } = useEvents();

  const [feeds, setFeeds] = useState<Feed[]>([]);
  const [editingFeed, setEditingFeed] = useState<Feed | null>(null);
  const [isAddingFeed, setIsAddingFeed] = useState(false);
  const [pinnedFeedIds, setPinnedFeedIds] = useLocalStorage<string[]>(
    "sidebar.pinnedFeeds",
    []
  );

  const requestFeeds = useCallback(async () => {
    const results = await listFeeds(sortByUnreadCount);
    if (results) setFeeds(results);
  }, [listFeeds]);

  const handleSaveFeed = useCallback(
    async (title: string, pinned: boolean) => {
      if (!editingFeed) return;
      await renameFeed(editingFeed.id, title);
      setPinnedFeedIds((prev) =>
        pinned
          ? prev.includes(editingFeed.id)
            ? prev
            : [...prev, editingFeed.id]
          : prev.filter((id) => id !== editingFeed.id)
      );
      await requestFeeds();
    },
    [editingFeed, renameFeed, setPinnedFeedIds, requestFeeds]
  );

  const handleDeleteFeed = useCallback(
    async (feed: Feed) => {
      await deleteFeed(feed.url);
      setPinnedFeedIds((prev) => prev.filter((id) => id !== feed.id));
      await requestFeeds();
    },
    [deleteFeed, setPinnedFeedIds, requestFeeds]
  );

  const handleAddFeed = useCallback(
    async (url: string, title: string, pinned: boolean) => {
      const feed = await addFeed(url);
      if (feed) {
        if (title) await renameFeed(feed.id, title);
        if (pinned) setPinnedFeedIds((prev) => [...prev, feed.id]);
        await requestFeeds();
        navigate(`/${feed.id}?filter=unread`);
      }
    },
    [addFeed, renameFeed, setPinnedFeedIds, requestFeeds, navigate]
  );

  const handleImportFeeds = useCallback(async () => {
    const result = await importFeeds();
    if (result !== null && result !== undefined) await requestFeeds();
  }, [importFeeds, requestFeeds]);

  const handleExportFeeds = useCallback(async () => {
    await exportFeeds();
  }, [exportFeeds]);

  useEffect(
    () => onUnreadChanged(() => requestFeeds()),
    [onUnreadChanged, requestFeeds]
  );

  useEffect(() => {
    const unlisten = platform.onImportFeeds(handleImportFeeds);
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [platform, handleImportFeeds]);

  useEffect(() => {
    const unlisten = platform.onExportFeeds(handleExportFeeds);
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [platform, handleExportFeeds]);

  useEffect(() => {
    abort();
    requestFeeds();
  }, [abort, requestFeeds]);

  const value = useMemo<ManageFeedContextType>(
    () => ({
      feeds,
      pinnedFeedIds,
      loadingFeedIds: loadingSet,
      startEditingFeed: setEditingFeed,
      startAddingFeed: () => setIsAddingFeed(true),
    }),
    [feeds, pinnedFeedIds, loadingSet]
  );

  return (
    <ManageFeedContext.Provider value={value}>
      {children}
      <EditFeedDialog
        open={editingFeed !== null || isAddingFeed}
        feed={editingFeed}
        pinned={pinnedFeedIds.includes(editingFeed?.id ?? "")}
        onClose={() => {
          setEditingFeed(null);
          setIsAddingFeed(false);
        }}
        onAdd={handleAddFeed}
        onSave={handleSaveFeed}
        onDelete={handleDeleteFeed}
      />
    </ManageFeedContext.Provider>
  );
}

export function useManageFeed() {
  const ctx = useContext(ManageFeedContext);
  if (!ctx)
    throw new Error("useManageFeed must be used inside ManageFeedProvider");
  return ctx;
}
