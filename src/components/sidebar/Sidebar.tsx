import { Add, Feed as FeedIcon, Newspaper, Star } from "@mui/icons-material";
import { IconButton, Typography } from "@mui/material";
import { useLocalStorage } from "../../hooks/useLocalStorage";
import { useManageFeed } from "../../providers/ManageFeedProvider";
import { useManageTopic } from "../../providers/ManageTopicProvider";
import {
  ALL_ITEMS,
  SAVED_ITEMS,
  SIMILAR_TO_SAVED_ITEMS,
} from "../../Shortcuts";
import {
  isShortcut,
  isTopic,
  type Collection,
  type Feed,
  type Item,
  type Topic,
} from "../../types";
import cx from "../../utils/cx";
import css from "./_.module.css";
import Expander from "./Expander";
import ExpanderItem from "./ExpanderItem";

export const SIDEBAR_WIDTH = 240;

interface SidebarProps {
  className?: string;
  selectedCollection: Collection;
  onSelectCollection: (collection: Collection) => void;
}

export type ExpanderType = "shortcuts" | "topics" | "projects" | "feeds";

const sortByTitle = (a: Item, b: Item) => a.title.localeCompare(b.title);

const sortByUnreadCount = (a: Feed | Topic, b: Feed | Topic) =>
  b.unreadCount - a.unreadCount || sortByTitle(a, b);

export function Sidebar({
  className,
  selectedCollection,
  onSelectCollection,
}: SidebarProps) {
  const [selectedExpander, setSelectedExpander] =
    useLocalStorage<ExpanderType | null>("sidebar.expander", "shortcuts");

  const { topics, pinnedTopicIds, startAddingTopic } = useManageTopic();
  const { feeds, pinnedFeedIds, loadingFeedIds, startAddingFeed } =
    useManageFeed();

  const getExpandHandler = (expander: ExpanderType) => () =>
    setSelectedExpander((prev) => (prev === expander ? null : expander));

  const pinnedCollections = [...pinnedTopicIds, ...pinnedFeedIds]
    .map((id) => [...topics, ...feeds].find((c) => c.id === id))
    .filter((c): c is Topic | Feed => c !== undefined)
    .sort(sortByUnreadCount);

  return (
    <div className={cx(className, css.Sidebar)}>
      <Expander
        label="Shortcuts"
        isExpanded={selectedExpander === "shortcuts"}
        onClickHeader={getExpandHandler("shortcuts")}
      >
        <ExpanderItem
          icon={Newspaper}
          item={ALL_ITEMS}
          loading={!!loadingFeedIds?.size}
          selected={
            isShortcut(selectedCollection) &&
            selectedCollection.id === ALL_ITEMS.id &&
            selectedCollection.filter === ALL_ITEMS.filter
          }
          onSelectCollection={onSelectCollection}
        />

        <ExpanderItem
          icon={Star}
          item={SIMILAR_TO_SAVED_ITEMS}
          loading={!!loadingFeedIds?.size}
          selected={
            isShortcut(selectedCollection) &&
            selectedCollection.id === SIMILAR_TO_SAVED_ITEMS.id &&
            selectedCollection.filter === SIMILAR_TO_SAVED_ITEMS.filter
          }
          onSelectCollection={onSelectCollection}
        />

        <ExpanderItem
          icon={Star}
          item={SAVED_ITEMS}
          loading={!!loadingFeedIds?.size}
          selected={
            isShortcut(selectedCollection) &&
            selectedCollection.id === SAVED_ITEMS.id &&
            selectedCollection.filter === SAVED_ITEMS.filter
          }
          onSelectCollection={onSelectCollection}
        />

        {pinnedCollections.map((item) => (
          <ExpanderItem
            key={item.id}
            icon={isTopic(item) ? FeedIcon : undefined}
            item={item}
            loading={!!loadingFeedIds?.size}
            selected={selectedCollection?.id === item.id}
            onSelectCollection={onSelectCollection}
            renderSecondaryAction={() =>
              item.unreadCount > 0 ? (
                <Typography
                  variant="caption"
                  color="text.secondary"
                  sx={{ pr: 1 }}
                >
                  {item.unreadCount}
                </Typography>
              ) : null
            }
          />
        ))}
      </Expander>

      <Expander
        label="Topics"
        isExpanded={selectedExpander === "topics"}
        onClickHeader={getExpandHandler("topics")}
        renderHeaderAction={() => (
          <IconButton size="small" onClick={startAddingTopic}>
            <Add sx={{ fontSize: 16 }} />
          </IconButton>
        )}
      >
        {topics.map((item) => (
          <ExpanderItem
            key={item.id}
            icon={FeedIcon}
            item={item}
            loading={!!loadingFeedIds?.size}
            selected={selectedCollection?.id === item.id}
            onSelectCollection={onSelectCollection}
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

      <Expander
        label="Feeds"
        isExpanded={selectedExpander === "feeds"}
        onClickHeader={getExpandHandler("feeds")}
        renderHeaderAction={() => (
          <IconButton size="small" onClick={startAddingFeed}>
            <Add sx={{ fontSize: 16 }} />
          </IconButton>
        )}
      >
        {feeds.map((feed) => (
          <ExpanderItem
            key={feed.id}
            item={feed}
            loading={loadingFeedIds?.has(feed.id)}
            selected={selectedCollection?.id === feed.id}
            onSelectCollection={onSelectCollection}
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
    </div>
  );
}
