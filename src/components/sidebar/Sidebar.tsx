import {
  Add,
  Feed as FeedIcon,
  FormatQuote,
  Newspaper,
  Star
} from "@mui/icons-material";
import { IconButton, Typography } from "@mui/material";
import { useCallback, useEffect, useState } from "react";
import useAbortController from "../../hooks/useAbortController";
import { useEvents } from "../../hooks/useEvents";
import { useFeeds } from "../../hooks/useFeeds";
import { useLocalStorage } from "../../hooks/useLocalStorage";
import { useTopics } from "../../hooks/useTopics";
import { usePlatform } from "../../platform/Platform";
import {
  ALL_ITEMS,
  ANNOTATION_ITEMS,
  RECOMMENDED_ITEMS
} from "../../Shortcuts";
import { isShortcut, type Feed, type Item, type Topic } from "../../types";
import cx from "../../utils/cx";
import css from "./_.module.css";
import EditFeedDialog from "./EditFeedDialog";
import EditTopicDialog from "./EditTopicDialog";
import Expander from "./Expander";
import ExpanderItem from "./ExpanderItem";

export const SIDEBAR_WIDTH = 240;

interface SidebarProps {
  className?: string;
  selectedList: Item;
  onSelectList: (item: Item) => void;
  editingFeed: Feed | null;
  onSetEditingFeed: (feed: Feed | null) => void;
  editingTopic: Topic | null;
  onSetEditingTopic: (topic: Topic | null) => void;
}

export type ExpanderType = "shortcuts" | "topics" | "projects" | "feeds";

// TODO Allow user to sort
const sortByTitle = (a: Item, b: Item) => a.title.localeCompare(b.title);

// const sortByUpdatedAt = (a: Feed | Topic, b: Feed | Topic) =>
//   compareDates(b.updatedAt, a.updatedAt);

const sortByUnreadCount = (a: Feed | Topic, b: Feed | Topic) =>
  b.unreadCount - a.unreadCount || sortByTitle(a, b);

export function Sidebar({
  className,
  selectedList,
  onSelectList,
  editingFeed,
  onSetEditingFeed,
  editingTopic,
  onSetEditingTopic,
}: SidebarProps) {
  const platform = usePlatform();
  const { onUnreadChanged, onEmbeddingsChanged } = useEvents();
  const [abort, abortControllerRef] = useAbortController();

  const [selectedExpander, setSelectedExpander] = useLocalStorage<ExpanderType | null>(
    "sidebar.expander",
    "shortcuts",
  );

  const { listTopics, saveTopic, createTopic, deleteTopic } =
    useTopics(abortControllerRef);

  const {
    listFeeds,
    addFeed,
    importFeeds,
    exportFeeds,
    renameFeed,
    deleteFeed,
  } = useFeeds(abortControllerRef);

  const { loadingSet } = useFeeds(abortControllerRef);
  const [topics, setTopics] = useState<Topic[]>([]);
  const [feeds, setFeeds] = useState<Feed[]>([]);

  const [isAddingTopic, setIsAddingTopic] = useState(false);
  const setEditingFeed = onSetEditingFeed;
  const [isAddingFeed, setIsAddingFeed] = useState(false);

  const getExpandHandler = useCallback((expander: ExpanderType) => {
    return () => {
      setSelectedExpander((prev) => (prev === expander ? null : expander));
    };
  }, [setSelectedExpander]);

  const requestFeeds = useCallback(async () => {
    const results = await listFeeds(sortByUnreadCount);
    if (results) setFeeds(results);
  }, [listFeeds]);

  const requestTopics = useCallback(async () => {
    const results = await listTopics(sortByUnreadCount);
    if (results) setTopics(results);
  }, [listTopics]);

  const handleSaveTopic = useCallback(
    async (title: string, content: string, threshold: number) => {
      let topic: Topic | undefined;

      if (editingTopic) {
        topic = await saveTopic({ ...editingTopic, title, content, threshold });
      } else {
        topic = await createTopic({ title, content, threshold });
      }

      if (topic) {
        await requestTopics();
        onSelectList(topic);
      }
    },
    [createTopic, editingTopic, onSelectList, requestTopics, saveTopic],
  );

  const handleDeleteTopic = useCallback(
    async (topic: Topic) => {
      await deleteTopic(topic.id);
      await requestTopics();
    },
    [deleteTopic, requestTopics],
  );

  const handleSaveFeed = useCallback(
    async (title: string) => {
      if (!editingFeed) return;
      await renameFeed(editingFeed.id, title);
      await requestFeeds();
    },
    [editingFeed, renameFeed, requestFeeds],
  );

  const handleDeleteFeed = useCallback(
    async (feed: Feed) => {
      await deleteFeed(feed.url);
      await requestFeeds();
    },
    [deleteFeed, requestFeeds],
  );

  const handleAddFeed = useCallback(
    async (url: string, title: string) => {
      const feed = await addFeed(url);
      if (feed) {
        if (title) await renameFeed(feed.id, title);
        await requestFeeds();
        onSelectList(feed);
      }
    },
    [addFeed, renameFeed, requestFeeds, onSelectList],
  );

  const handleImportFeeds = useCallback(async () => {
    const result = await importFeeds();
    if (result !== null && result !== undefined) {
      await requestFeeds();
    }
  }, [importFeeds, requestFeeds]);

  const handleExportFeeds = useCallback(async () => {
    await exportFeeds();
  }, [exportFeeds]);

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
    const unlisten = onUnreadChanged(() => {
      requestTopics();
      requestFeeds();
    });
    return () => {
      unlisten();
    };
  }, [onUnreadChanged, requestTopics, requestFeeds]);

  useEffect(() => {
    const unlisten = onEmbeddingsChanged(() => {
      requestTopics();
    });
    return () => {
      unlisten();
    };
  }, [onEmbeddingsChanged, requestTopics]);

  useEffect(() => {
    abort();

    (async () => {
      requestTopics();
      requestFeeds();
    })();
  }, [abort, requestTopics, requestFeeds]);

  return (
    <div className={cx(className, css.Sidebar)}>
      {/* <Shortcuts onSelectList={onSelectList} /> */}

      <Expander
        label="Shortcuts"
        isExpanded={selectedExpander === "shortcuts"}
        onClickHeader={getExpandHandler("shortcuts")}
      // TODO Implement "Add shortcut" action
      // renderHeaderAction={() => (
      //   <IconButton size="small" onClick={() => setIsAddingTopic(true)}>
      //     <Add sx={{ fontSize: 16 }} />
      //   </IconButton>
      // )}
      >
        <ExpanderItem
          icon={Newspaper}
          item={ALL_ITEMS}
          loading={!!loadingSet?.size}
          selected={
            isShortcut(selectedList) &&
            selectedList.id === ALL_ITEMS.id &&
            selectedList.filter === ALL_ITEMS.filter
          }
          onSelectItem={onSelectList}
        />

        <ExpanderItem
          icon={Star}
          item={RECOMMENDED_ITEMS}
          loading={!!loadingSet?.size}
          selected={
            isShortcut(selectedList) &&
            selectedList.id === RECOMMENDED_ITEMS.id &&
            selectedList.filter === RECOMMENDED_ITEMS.filter
          }
          onSelectItem={onSelectList}
        />

        <ExpanderItem
          icon={FormatQuote}
          item={ANNOTATION_ITEMS}
          loading={!!loadingSet?.size}
          selected={
            isShortcut(selectedList) &&
            selectedList.id === ANNOTATION_ITEMS.id &&
            selectedList.filter === ANNOTATION_ITEMS.filter
          }
          onSelectItem={onSelectList}
        />
      </Expander>

      <Expander
        label="Topics"
        isExpanded={selectedExpander === "topics"}
        onClickHeader={getExpandHandler("topics")}
        renderHeaderAction={() => (
          <IconButton size="small" onClick={() => setIsAddingTopic(true)}>
            <Add sx={{ fontSize: 16 }} />
          </IconButton>
        )}
      >
        {topics.map((item) => (
          <ExpanderItem
            icon={FeedIcon}
            item={item}
            loading={!!loadingSet?.size}
            selected={selectedList?.id === item.id}
            onSelectItem={onSelectList}
            renderSecondaryAction={(item: Item) => {
              const topic = item as Topic;
              return topic.unreadCount > 0 ? (
                <Typography
                  variant="caption"
                  color="text.secondary"
                  sx={{ pr: 1 }}
                >
                  {topic.unreadCount}
                </Typography>
              ) : null;
            }}
          />
        ))}
      </Expander>

      <EditTopicDialog
        open={editingTopic !== null || isAddingTopic}
        topic={editingTopic}
        onClose={() => {
          onSetEditingTopic(null);
          setIsAddingTopic(false);
        }}
        onSave={handleSaveTopic}
        onDelete={handleDeleteTopic}
      />

      <Expander
        label="Feeds"
        isExpanded={selectedExpander === "feeds"}
        onClickHeader={getExpandHandler("feeds")}
        renderHeaderAction={() => (
          <IconButton size="small" onClick={() => setIsAddingFeed(true)}>
            <Add sx={{ fontSize: 16 }} />
          </IconButton>
        )}
      >
        {feeds.map((feed) => (
          <ExpanderItem
            key={feed.id}
            item={feed}
            loading={loadingSet?.has(feed.id)}
            selected={selectedList?.id === feed.id}
            onSelectItem={onSelectList}
            renderSecondaryAction={() =>
              feed.unreadCount > 0 ? (
                <Typography
                  variant="caption"
                  color="text.secondary"
                  sx={{ pr: 1 }}
                >
                  {feed.unreadCount}
                </Typography>
              ) : null
            }
          />
        ))}
      </Expander>

      <EditFeedDialog
        open={editingFeed !== null || isAddingFeed}
        feed={editingFeed}
        onClose={() => {
          setEditingFeed(null);
          setIsAddingFeed(false);
        }}
        onAdd={handleAddFeed}
        onSave={handleSaveFeed}
        onDelete={handleDeleteFeed}
      />
    </div>
  );
}
