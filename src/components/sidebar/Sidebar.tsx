import {
  Add,
  Bookmark,
  Circle,
  Feed as FeedIcon,
  Settings,
  Star,
} from "@mui/icons-material";
import { IconButton } from "@mui/material";
import { useCallback, useEffect, useState } from "react";
import useAbortController from "../../hooks/useAbortController";
import { useFeeds } from "../../hooks/useFeeds";
import { useTopics } from "../../hooks/useTopics";
import {
  BOOKMARKED_ITEMS,
  RECOMMENDED_ITEMS,
  UNREAD_ITEMS,
} from "../../Shortcuts";
import { usePlatform } from "../../platform/Platform";
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
}

export type ExpanderType = "shortcuts" | "topics" | "projects" | "feeds";

// TODO Allow user to sort
// const sort = (a: Feed, b: Feed) => compareDates(b.updatedAt, a.updatedAt);
const sort = (a: Item, b: Item) => a.title.localeCompare(b.title);

export function Sidebar({
  className,
  selectedList,
  onSelectList,
}: SidebarProps) {
  const platform = usePlatform();
  const [abort, abortControllerRef] = useAbortController();

  const [selectedExpander, setSelectedExpander] = useState<ExpanderType | null>(
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

  const [editingTopic, setEditingTopic] = useState<Topic | null>(null);
  const [isAddingTopic, setIsAddingTopic] = useState(false);
  const [editingFeed, setEditingFeed] = useState<Feed | null>(null);
  const [isAddingFeed, setIsAddingFeed] = useState(false);

  const getExpandHandler = useCallback((expander: ExpanderType) => {
    return () => {
      setSelectedExpander((prev) => (prev === expander ? null : expander));
    };
  }, []);

  const requestFeeds = useCallback(async () => {
    const results = await listFeeds(sort);
    if (results) setFeeds(results);
  }, [listFeeds]);

  const requestTopics = useCallback(async () => {
    const results = await listTopics(sort);
    if (results) setTopics(results);
  }, [listTopics]);

  const handleSaveTopic = useCallback(
    async (title: string, content: string) => {
      let topic: Topic | undefined;

      if (editingTopic) {
        topic = await saveTopic({ ...editingTopic, title, content });
      } else {
        topic = await createTopic({ title, content });
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
          icon={Circle}
          item={UNREAD_ITEMS}
          loading={!!loadingSet?.size}
          selected={
            isShortcut(selectedList) &&
            selectedList.id === UNREAD_ITEMS.id &&
            selectedList.filter === UNREAD_ITEMS.filter
          }
          onSelectItem={onSelectList}
        />

        <ExpanderItem
          icon={Bookmark}
          item={BOOKMARKED_ITEMS}
          loading={!!loadingSet?.size}
          selected={
            isShortcut(selectedList) &&
            selectedList.id === BOOKMARKED_ITEMS.id &&
            selectedList.filter === BOOKMARKED_ITEMS.filter
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
            renderSecondaryAction={(item: Item) => (
              <IconButton
                size="small"
                onClick={(e) => {
                  e.stopPropagation();
                  setEditingTopic(item as Topic);
                }}
              >
                <Settings sx={{ fontSize: 16 }} />
              </IconButton>
            )}
          />
        ))}
      </Expander>

      <EditTopicDialog
        open={editingTopic !== null || isAddingTopic}
        topic={editingTopic}
        onClose={() => {
          setEditingTopic(null);
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
        {feeds.map((item) => (
          <ExpanderItem
            item={item}
            loading={loadingSet?.has(item.id)}
            selected={selectedList?.id === item.id}
            onSelectItem={onSelectList}
            renderSecondaryAction={(item: Item) => (
              <IconButton
                size="small"
                onClick={(e) => {
                  e.stopPropagation();
                  setEditingFeed(item as Feed);
                }}
              >
                <Settings sx={{ fontSize: 16 }} />
              </IconButton>
            )}
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
