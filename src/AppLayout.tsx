import { Collapse } from "@mui/material";
import { useCallback, useEffect, useState } from "react";
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
import { useFeeds } from "./hooks/useFeeds";
import { useTopics } from "./hooks/useTopics";
import { isFilter, type Feed, type Topic } from "./types";

const storedFilter = (() => {
  try {
    const filter = localStorage.getItem("list.filter");
    return isFilter(filter) ? filter : "unread";
  } catch {
    return "unread";
  }
})();

export function AppLayout() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const filterParam = searchParams.get("filter");
  const filter = isFilter(filterParam) ? filterParam : storedFilter;
  const search = searchParams.get("search");

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
    if (filter) localStorage.setItem("list.filter", filter);
  }, [filter]);

  useEffect(() => {
    requestFeeds();
  }, [requestFeeds]);

  useEffect(() => {
    requestTopics();
  }, [requestTopics]);

  return (
    <div className={css.Root}>
      <TitleBar className={css.TitleBar} />

      <div className={css.App}>
        <Sidebar
          className={css.Sidebar}
          onAddFeed={() => setIsAddingFeed(true)}
          onAddTopic={() => setIsAddingTopic(true)}
        />

        <ItemList
          className={css.ItemList}
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
    </div>
  );
}
