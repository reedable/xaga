import { Clear, Settings } from "@mui/icons-material";
import {
  Button,
  Collapse,
  IconButton,
  InputAdornment,
  LinearProgress,
  ListItemButton,
  ListItemText,
  TextField,
  ToggleButton,
  ToggleButtonGroup,
  Typography,
} from "@mui/material";
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type MouseEvent,
} from "react";
import { Virtuoso } from "react-virtuoso";
import { RELATED_THRESHOLD, TOPIC_THRESHOLD } from "../../consts";
import useAbortController from "../../hooks/useAbortController";
import { useCollections } from "../../hooks/useCollections";
import useEntries from "../../hooks/useEntries";
import { useEvents } from "../../hooks/useEvents";
import { useFeeds } from "../../hooks/useFeeds";
import useRouter from "../../hooks/useRouter";
import useRelatedItems from "../../hooks/useRelatedItems";
import { useSaved } from "../../hooks/useSaved";
import { useTopics } from "../../hooks/useTopics";
import {
  ALL_ITEMS,
  ANNOTATION_ITEMS,
  RECOMMENDED_ITEMS,
  SAVED_ITEMS,
  SHORTCUTS,
  SIMILAR_TO_SAVED_ITEMS,
} from "../../Shortcuts";
import {
  isAnnotation,
  isEntry,
  isFeed,
  isFilter,
  isTopic,
  type Collection,
  type Feed,
  type Filter,
  type Item,
  type Topic,
} from "../../types";
import cx from "../../utils/cx";
import ItemStatus from "../ItemStatus";
import Quotes from "../Quotes";
import css from "./ItemList.module.css";

function getEmptyMessage(filter: Filter, filterText: string): string {
  const label = filter === "all" ? "items" : `${filter} items`;
  return filterText ? `No ${label} match "${filterText}"` : `No ${label}`;
}

type ItemListProps = {
  className?: string;
  // selectedCollection: Collection;
  onEditFeed: (feed: Feed) => void;
  onEditTopic: (topic: Topic) => void;
  onEditCollection: (collection: Collection) => void;
};

export function ItemList({
  className,
  // selectedCollection,
  onEditFeed,
  onEditTopic,
  onEditCollection,
}: ItemListProps) {
  const { show, state, searchParams, setSearchParams } = useRouter();
  const { feedId, topicId, shortcutId, collectionId, entryId: selectedItemId } = state;
  const filterParam = searchParams.get("filter");
  const filter: Filter = isFilter(filterParam) ? filterParam : "unread";

  const [abort, abortControllerRef] = useAbortController();
  const { topics } = useTopics(abortControllerRef);
  const { feeds } = useFeeds(abortControllerRef);
  const { collections, listCollectionItems } = useCollections(abortControllerRef);
  const [loading, setLoading] = useState(false);

  const selectedCollection = useMemo(
    () =>
      (feedId && feeds.find((f) => f.id === feedId)) ||
      (topicId && topics.find((t) => t.id === topicId)) ||
      (shortcutId && SHORTCUTS.find((s) => s.id === shortcutId)) ||
      (collectionId && collections.find((c) => c.id === collectionId)) ||
      ALL_ITEMS,
    [feedId, topicId, shortcutId, collectionId, feeds, topics, collections]
  );

  const { listEntries, listAllItems, markEntryRead } =
    useEntries(abortControllerRef);

  const { listSaved } = useSaved(abortControllerRef);

  const { queryByItem, queryByBookmarks, queryByAnnotations, queryBySaved } =
    useRelatedItems(abortControllerRef);

  const {
    loading: loadingFeeds,
    refreshFeed,
    refreshAllFeeds,
  } = useFeeds(abortControllerRef);

  const { onUnreadChanged, onBookmarkChanged } = useEvents();

  const [items, setItems] = useState<Item[]>([]);
  const [filterText, setFilterText] = useState("");

  const handleEdit = useMemo(() => {
    if (isFeed(selectedCollection)) return () => onEditFeed(selectedCollection);
    if (isTopic(selectedCollection)) return () => onEditTopic(selectedCollection);
    if (collectionId && selectedCollection.id === collectionId) return () => onEditCollection(selectedCollection);
    return undefined;
  }, [selectedCollection, collectionId, onEditFeed, onEditTopic, onEditCollection]);

  const displayItems = useMemo(() => {
    if (!filterText) return items;
    const lower = filterText.toLowerCase();
    return items.filter((item) => {
      if (item.title.toLowerCase().includes(lower)) return true;
      if (isEntry(item)) {
        if (item.url.toLowerCase().includes(lower)) return true;
        if (item.summary?.toLowerCase().includes(lower)) return true;
      }
      return false;
    });
  }, [items, filterText]);

  const requestItems = useCallback(
    async (selectedList: Item) => {
      setLoading(true);

      try {
        if (selectedList === RECOMMENDED_ITEMS) {
          const entries = await queryByBookmarks(filter, 0.82);
          if (entries) setItems(entries);
        } else if (selectedList === SAVED_ITEMS) {
          const entries = await listSaved();
          if (entries) setItems(entries);
        } else if (selectedList === ANNOTATION_ITEMS) {
          const entries = await queryByAnnotations(filter, RELATED_THRESHOLD);
          if (entries) setItems(entries);
        } else if (selectedList === SIMILAR_TO_SAVED_ITEMS) {
          const entries = await queryBySaved(filter, RELATED_THRESHOLD);
          if (entries) setItems(entries);
        } else if (isFeed(selectedList)) {
          const entries = await listEntries(selectedList, filter);
          if (entries) setItems(entries);
        } else if (isTopic(selectedList)) {
          const items = await queryByItem(
            selectedList.id,
            filter,
            selectedList.threshold ?? TOPIC_THRESHOLD
          );
          if (items) setItems(items);
        } else if (collectionId && selectedList.id === collectionId) {
          const items = await listCollectionItems(collectionId, filter);
          if (items) setItems(items);
        } else {
          const entries = await listAllItems(filter);
          if (entries) setItems(entries);
        }
      } finally {
        setLoading(false);
      }
    },
    [
      queryByBookmarks,
      filter,
      listSaved,
      queryByAnnotations,
      queryBySaved,
      listEntries,
      queryByItem,
      listAllItems,
      collectionId,
      listCollectionItems,
    ]
  );

  const refreshList = useCallback(
    async (list: Item = ALL_ITEMS) => {
      if (isFeed(list)) {
        await refreshFeed(list.id);
        await requestItems(list);
      } else if (collectionId && list.id === collectionId) {
        await requestItems(list);
      } else {
        await refreshAllFeeds(() => requestItems(list));
      }
    },
    [refreshAllFeeds, refreshFeed, requestItems, collectionId]
  );

  const handleFilter = useCallback(
    (_event: MouseEvent<HTMLElement>, value: string) => {
      if (isFilter(value)) {
        setSearchParams(
          (prev) => {
            prev.set("filter", value);
            return prev;
          },
          { state }
        );
      }
    },
    [setSearchParams, state]
  );

  const handleRefresh = useCallback(() => {
    abort();
    (async () => {
      refreshList(selectedCollection);
    })();
  }, [abort, refreshList, selectedCollection]);

  const handleMarkEntriesRead = useCallback(async () => {
    await Promise.all(
      displayItems
        .filter(isEntry)
        .map((entry) => entry.id)
        .map(markEntryRead)
    );
  }, [displayItems, markEntryRead]);

  const selectedCollectionRef = useRef(selectedCollection);
  selectedCollectionRef.current = selectedCollection;

  useEffect(() => {
    abort();
    (async () => {
      requestItems(selectedCollectionRef.current);
    })();
    // Depend on IDs, not objects, so feed/topic identity changes from
    // requestFeeds/requestTopics don't re-trigger a full list reload.
  }, [feedId, topicId, shortcutId, collectionId, requestItems, abort]);

  useEffect(() => {
    return onUnreadChanged((entry) => {
      setItems((prev) => prev.map((p) => (p.id === entry.id ? entry : p)));
    });
  }, [onUnreadChanged]);

  useEffect(() => {
    return onBookmarkChanged((entry) => {
      setItems((prev) => prev.map((p) => (p.id === entry.id ? entry : p)));
    });
  }, [onBookmarkChanged]);

  return (
    <div className={cx(className, css.ItemList)}>
      <div className={css.Header}>
        <div className={css.HeadingWrapper}>
          <Typography
            className={css.Heading}
            component="div"
            color="text.secondary"
            variant="overline"
            sx={{ userSelect: "none" }}
          >
            {selectedCollection?.title ?? "All items"}
          </Typography>

          {handleEdit && (
            <IconButton size="small" onClick={handleEdit}>
              <Settings sx={{ fontSize: 16 }} />
            </IconButton>
          )}
        </div>

        <ToggleButtonGroup
          aria-label="Filter"
          className={css.FilterButtonGroup}
          exclusive
          value={filter}
          onChange={handleFilter}
        >
          <ToggleButton size="small" value="unread" aria-label="Unread">
            Unread
          </ToggleButton>
          <ToggleButton size="small" value="saved" aria-label="Saved">
            Saved
          </ToggleButton>
          <ToggleButton size="small" value="all" aria-label="All">
            All
          </ToggleButton>
        </ToggleButtonGroup>

        <TextField
          className={css.FilterInput}
          size="small"
          fullWidth
          placeholder="Filter…"
          value={filterText}
          onChange={(e) => setFilterText(e.target.value)}
          slotProps={{
            input: {
              endAdornment: filterText ? (
                <InputAdornment position="end">
                  <IconButton
                    size="small"
                    aria-label="Clear filter"
                    onClick={() => setFilterText("")}
                    edge="end"
                  >
                    <Clear fontSize="small" />
                  </IconButton>
                </InputAdornment>
              ) : null,
            },
          }}
        />
      </div>

      <div>
        <Collapse in={loading}>
          <LinearProgress
            variant="indeterminate"
            color={loadingFeeds ? "warning" : "primary"}
          />
        </Collapse>
      </div>

      {!loading && items.length === 0 && (
        <div className={cx(css.List, css.Placeholder)}>
          <Typography component="div" variant="body2" color="text.disabled">
            {getEmptyMessage(filter, "")}
          </Typography>
        </div>
      )}

      {items.length > 0 && displayItems.length === 0 && (
        <div className={cx(css.List, css.Placeholder)}>
          <Typography component="div" variant="body2" color="text.disabled">
            {getEmptyMessage(filter, filterText)}
          </Typography>
        </div>
      )}

      {displayItems.length > 0 && (
        <Virtuoso
          className={css.List}
          data={displayItems}
          itemContent={(_index, item) => (
            <div key={item.id} className={css.ListItem}>
              <ListItemButton
                className={css.ListItemButton}
                selected={selectedItemId === item.id}
                onClick={() => show(item)}
              >
                <ListItemText
                  primary={
                    <Quotes quoted={isAnnotation(item)}>{item.title}</Quotes>
                  }
                  secondary={<ItemStatus item={item} />}
                  slotProps={{
                    primary: { className: css.Primary },
                    secondary: { className: css.Secondary },
                  }}
                />
              </ListItemButton>
            </div>
          )}
        />
      )}

      {loading && items.length === 0 && (
        <div className={cx(css.List, css.Placeholder)}>
          <Typography component="div" variant="body2" color="text.disabled">
            Loading...
          </Typography>
        </div>
      )}

      <div className={css.Toolbar}>
        <Button onClick={handleMarkEntriesRead}>Mark all read</Button>
        <Button onClick={handleRefresh}>Refresh</Button>
      </div>
    </div>
  );
}
