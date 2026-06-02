import {
  Button,
  Collapse,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
} from "@mui/material";
import { useCallback, useEffect, useRef, useState } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import css from "./AppLayout.module.css";
import { ItemView } from "./components/content/ItemView";
import EditFeedDialog from "./components/dialogs/EditFeedDialog";
import EditTopicDialog from "./components/dialogs/EditTopicDialog";
import { ItemList } from "./components/list/ItemList";
import { SearchResultList } from "./components/search/SearchResultList";
import { Sidebar } from "./components/sidebar/Sidebar";
import { TitleBar } from "./components/titlebar/TitleBar";
import useAbortController from "./hooks/useAbortController";
import { usePlatform } from "./platform/Platform";
import { useAppearance } from "./hooks/useAppearance";
import { useFeeds } from "./hooks/useFeeds";
import { useTopics } from "./hooks/useTopics";
import { isFilter, type Feed, type Topic } from "./types";

export function AppLayout() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const appRef = useRef<HTMLDivElement>(null);
  const [autoHideSidebar, setAutoHideSidebar] = useState(false);
  const { showSidebar } = useAppearance();
  const filterParam = searchParams.get("filter");
  const [storedFilter] = useState<string>("unread");
  const filter = isFilter(filterParam) ? filterParam : storedFilter;
  const search = searchParams.get("search");

  const platform = usePlatform();
  const [clearHistoryOpen, setClearHistoryOpen] = useState(false);

  const [, abortControllerRef] = useAbortController();
  const [editingTopic, setEditingTopic] = useState<Topic | null>(null);
  const [isAddingTopic, setIsAddingTopic] = useState(false);
  const [editingFeed, setEditingFeed] = useState<Feed | null>(null);
  const [isAddingFeed, setIsAddingFeed] = useState(false);

  const {
    requestTopics,
    createTopic,
    saveTopic,
    deleteTopic,
    setPinnedTopicIds,
  } = useTopics(abortControllerRef);

  const { requestFeeds, addFeed, renameFeed, deleteFeed, setPinnedFeedIds } =
    useFeeds(abortControllerRef);

  const closeEditTopic = useCallback(() => {
    setEditingTopic(null);
    setIsAddingTopic(false);
  }, []);

  const handleSaveTopic = useCallback(
    async (
      title: string,
      content: string,
      threshold: number,
      pinned: boolean
    ) => {
      let topic: Topic | undefined;

      if (editingTopic) {
        topic = await saveTopic({ ...editingTopic, title, content, threshold });
      } else {
        topic = await createTopic({ title, content, threshold });
      }

      if (topic) {
        setPinnedTopicIds((prev) =>
          pinned
            ? prev.includes(topic.id)
              ? prev
              : [...prev, topic.id]
            : prev.filter((id) => id !== topic.id)
        );
        await requestTopics();
        navigate("/?filter=unread", { state: { topicId: topic.id } });
      }
    },
    [
      editingTopic,
      saveTopic,
      createTopic,
      setPinnedTopicIds,
      requestTopics,
      navigate,
    ]
  );

  const handleDeleteTopic = useCallback(
    async (topic: Topic) => {
      await deleteTopic(topic.id);
      setPinnedTopicIds((prev) => prev.filter((id) => id !== topic.id));
      await requestTopics();
    },
    [deleteTopic, setPinnedTopicIds, requestTopics]
  );

  const closeEditFeed = useCallback(() => {
    setEditingFeed(null);
    setIsAddingFeed(false);
  }, []);

  const handleAddFeed = useCallback(
    async (url: string, title: string, pinned: boolean) => {
      const feed = await addFeed(url);
      if (feed) {
        if (title) await renameFeed(feed.id, title);
        if (pinned) setPinnedFeedIds((prev) => [...prev, feed.id]);
        await requestFeeds();
        navigate("/?filter=unread", { state: { feedId: feed.id } });
      }
    },
    [addFeed, renameFeed, setPinnedFeedIds, requestFeeds, navigate]
  );

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
    async (feed: { url: string; id: string }) => {
      await deleteFeed(feed.url);
      setPinnedFeedIds((prev) => prev.filter((id) => id !== feed.id));
      await requestFeeds();
    },
    [deleteFeed, setPinnedFeedIds, requestFeeds]
  );

  useEffect(() => {
    const el = appRef.current;
    if (!el) return;
    const observer = new ResizeObserver(([entry]) => {
      const threshold = search !== null ? 1300 : 980;
      setAutoHideSidebar(entry.contentRect.width < threshold);
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, [search]);

  useEffect(() => {
    requestFeeds();
  }, [requestFeeds]);

  useEffect(() => {
    requestTopics();
  }, [requestTopics]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    platform.onClearHistory(() => setClearHistoryOpen(true)).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, [platform]);

  return (
    <div className={css.Root}>
      <TitleBar className={css.TitleBar} />

      <div ref={appRef} className={css.App}>
        <Collapse
          in={showSidebar && !autoHideSidebar}
          unmountOnExit
          orientation="horizontal"
        >
          <Sidebar
            className={css.Sidebar}
            onAddFeed={() => setIsAddingFeed(true)}
            onAddTopic={() => setIsAddingTopic(true)}
          />
        </Collapse>

        <ItemList
          className={css.ItemList}
          isSelectableHeader={autoHideSidebar || !showSidebar}
          onEditFeed={setEditingFeed}
          onEditTopic={setEditingTopic}
        />

        <ItemView className={css.ItemView} />

        <Collapse in={search !== null} unmountOnExit orientation="horizontal">
          <SearchResultList
            className={css.SearchResultList}
            searchQuery={search ?? ""}
          />
        </Collapse>
      </div>

      <EditTopicDialog
        open={editingTopic !== null || isAddingTopic}
        topic={editingTopic}
        onClose={closeEditTopic}
        onSave={handleSaveTopic}
        onDelete={handleDeleteTopic}
      />

      <EditFeedDialog
        open={editingFeed !== null || isAddingFeed}
        feed={editingFeed}
        onClose={closeEditFeed}
        onAdd={handleAddFeed}
        onSave={handleSaveFeed}
        onDelete={handleDeleteFeed}
      />

      <Dialog
        open={clearHistoryOpen}
        onClose={() => setClearHistoryOpen(false)}
      >
        <DialogTitle>Clear History</DialogTitle>
        <DialogContent>
          <DialogContentText>
            This will clear all cookies and website data from external sites
            visited in Xaga. Xaga&apos;s feeds, entries, and settings will not
            be affected.
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setClearHistoryOpen(false)}>Cancel</Button>
          <Button
            color="error"
            onClick={async () => {
              await platform.clearWebviewHistory();
              setClearHistoryOpen(false);
            }}
          >
            Clear History
          </Button>
        </DialogActions>
      </Dialog>
    </div>
  );
}
