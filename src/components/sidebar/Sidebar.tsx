import { Add, Settings } from "@mui/icons-material";
import { IconButton } from "@mui/material";
import { useCallback, useEffect, useState } from "react";
import useAbortController from "../../hooks/useAbortController";
import { useFeeds } from "../../hooks/useFeeds";
import { useTopics } from "../../hooks/useTopics";
import type { Feed, Filter, Item, Topic } from "../../types";
import compareDates from "../../utils/compareDates";
import cx from "../../utils/cx";
import css from "./_.module.css";
import EditFeedDialog from "./EditFeedDialog";
import EditTopicDialog from "./EditTopicDialog";
import Expander from "./Expander";
import { Shortcuts } from "./Shortcuts";

export const SIDEBAR_WIDTH = 240;

interface SidebarProps {
  className?: string;
  selectedList: Item;
  onSelectList: (item: Item, filter?: Filter) => void;
}

export type ExpanderType = "topics" | "projects" | "feeds";

const sort = (a: Feed, b: Feed) => compareDates(b.updatedAt, a.updatedAt);

export function Sidebar({
  className,
  selectedList,
  onSelectList,
}: SidebarProps) {
  const [abort, abortControllerRef] = useAbortController();
  const [selectedExpander, setSelectedExpander] = useState<ExpanderType | null>(
    null,
  );
  const { listTopics, saveTopic, createTopic, deleteTopic } =
    useTopics(abortControllerRef);
  const { listFeeds, addFeed, renameFeed, deleteFeed } = useFeeds(abortControllerRef);
  const [editingTopic, setEditingTopic] = useState<Topic | null>(null);
  const [isAddingTopic, setIsAddingTopic] = useState(false);
  const [editingFeed, setEditingFeed] = useState<Feed | null>(null);
  const [isAddingFeed, setIsAddingFeed] = useState(false);

  const [topics, setTopics] = useState<Topic[]>([]);
  const [feeds, setFeeds] = useState<Feed[]>([]);

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
    const results = await listTopics();
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

  useEffect(() => {
    abort();

    (async () => {
      requestTopics();
      requestFeeds();
    })();
  }, [abort, requestTopics, requestFeeds]);

  return (
    <div className={cx(className, css.Sidebar)}>
      <Shortcuts onSelectList={onSelectList} />

      <Expander
        label="Topics"
        items={topics}
        isExpanded={selectedExpander === "topics"}
        selectedList={selectedList}
        onClickHeader={getExpandHandler("topics")}
        onSelectList={onSelectList}
        renderHeaderAction={() => (
          <IconButton size="small" onClick={() => setIsAddingTopic(true)}>
            <Add sx={{ fontSize: 16 }} />
          </IconButton>
        )}
        renderSecondaryAction={(item) => (
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

      {/* <Expander
        label="Projects"
        items={projects}
        isExpanded={selectedExpander === "projects"}
        selectedItem={selectedList}
        onClickHeader={getExpandHandler("projects")}
        onSelectItem={onSelectItem}
      /> */}

      <Expander
        label="Feeds"
        items={feeds}
        isExpanded={selectedExpander === "feeds"}
        selectedList={selectedList}
        onClickHeader={getExpandHandler("feeds")}
        onSelectList={onSelectList}
        renderHeaderAction={() => (
          <IconButton size="small" onClick={() => setIsAddingFeed(true)}>
            <Add sx={{ fontSize: 16 }} />
          </IconButton>
        )}
        renderSecondaryAction={(item) => (
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
