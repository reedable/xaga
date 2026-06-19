import { Clear, Refresh, Settings } from "@mui/icons-material";
import {
  Button,
  Collapse,
  IconButton,
  InputAdornment,
  LinearProgress,
  ListItemButton,
  ListItemText,
  ListSubheader,
  MenuItem,
  Select,
  TextField,
  ToggleButton,
  ToggleButtonGroup,
  Tooltip,
  Typography,
  type SelectChangeEvent,
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
import useAbortController from "../../hooks/useAbortController";
import useEntries from "../../hooks/useEntries";
import { useEvents } from "../../hooks/useEvents";
import { useFeeds } from "../../hooks/useFeeds";
import useRouter from "../../hooks/useRouter";
import { useTopics } from "../../hooks/useTopics";
import { ALL_ITEMS, SHORTCUTS } from "../../Shortcuts";
import {
  isAnnotation,
  isEntry,
  isFeed,
  isTopic,
  type Feed,
  type Filter,
  type Item,
  type Topic,
  type TopicSort,
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
  isSelectableHeader: boolean;
  onEditFeed: (feed: Feed) => void;
  onEditTopic: (topic: Topic) => void;
};

export function ItemList({
  className,
  isSelectableHeader,
  onEditFeed,
  onEditTopic,
}: ItemListProps) {
  const { filter, selectItem, state, selectFilter } = useRouter();
  const { feedId, topicId, shortcutId, entryId: selectedItemId } = state;
  const [abort, abortControllerRef] = useAbortController();
  const { topics, listTopicItems } = useTopics(abortControllerRef);

  const {
    feeds,
    loading: loadingFeeds,
    refreshAllFeeds,
  } = useFeeds(abortControllerRef);

  const [loading, setLoading] = useState(false);
  const [refreshSpin, setRefreshSpin] = useState(false);

  const selectedCollection = useMemo(
    () =>
      (feedId && feeds.find((f) => f.id === feedId)) ||
      (topicId && topics.find((t) => t.id === topicId)) ||
      (shortcutId && SHORTCUTS.find((s) => s.id === shortcutId)) ||
      ALL_ITEMS,
    [feedId, topicId, shortcutId, feeds, topics]
  );

  const { listEntries, listAllItems, markEntryRead } =
    useEntries(abortControllerRef);

  const {
    onFeedRefreshed,
    onUnreadChanged,
    onBookmarkChanged,
    onAnnotationChanged,
    onAnnotationRemoved,
  } = useEvents();

  const [items, setItems] = useState<Item[]>([]);
  const [filterText, setFilterText] = useState("");
  const [topicSort, setTopicSort] = useState<TopicSort>("relevance");

  const handleEdit = useMemo(() => {
    if (isFeed(selectedCollection)) return () => onEditFeed(selectedCollection);
    if (isTopic(selectedCollection))
      return () => onEditTopic(selectedCollection);
    return undefined;
  }, [selectedCollection, onEditFeed, onEditTopic]);

  const handleCollectionChange = useCallback(
    (event: SelectChangeEvent<string>) => {
      const id = event.target.value;
      const item = items.find((i) => i.id === id);
      if (item) selectItem(item);
    },
    [items, selectItem]
  );

  const handleRefresh = useCallback(() => {
    refreshAllFeeds();
    setRefreshSpin(true);
  }, [refreshAllFeeds]);

  useEffect(() => {
    if (!loadingFeeds) setRefreshSpin(false);
  }, [loadingFeeds]);

  const displayItems = useMemo(() => {
    if (!filterText) return items;
    const lower = filterText.toLowerCase();
    return items.filter((item) => {
      if (item.title.toLowerCase().includes(lower)) return true;
      if (isEntry(item)) {
        if (item.url.toLowerCase().includes(lower)) return true;
        if (item.description?.toLowerCase().includes(lower)) return true;
      }
      return false;
    });
  }, [items, filterText]);

  const requestItems = useCallback(
    async (selectedList: Item) => {
      setLoading(true);

      try {
        if (isFeed(selectedList)) {
          const entries = await listEntries(selectedList, filter);
          if (entries) setItems(entries);
        } else if (isTopic(selectedList)) {
          const items = await listTopicItems(
            selectedList.id,
            filter ?? "unread",
            topicSort
          );
          if (items) setItems(items);
        } else {
          const entries = await listAllItems(filter);
          if (entries) setItems(entries);
        }
      } finally {
        setLoading(false);
      }
    },
    [filter, listEntries, listTopicItems, listAllItems, topicSort]
  );

  const handleFilter = useCallback(
    (_event: MouseEvent<HTMLElement>, value: string) => {
      selectFilter(value);
    },
    [selectFilter]
  );

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
  }, [feedId, topicId, shortcutId, requestItems, abort]);

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

  useEffect(() => {
    return onAnnotationChanged((item) => {
      setItems((prev) => prev.map((p) => (p.id === item.id ? item : p)));
    });
  }, [onAnnotationChanged]);

  useEffect(() => {
    return onAnnotationRemoved((id) => {
      setItems((prev) => prev.filter((p) => p.id !== id));
    });
  }, [onAnnotationRemoved]);

  useEffect(() => {
    return onFeedRefreshed(() => requestItems(selectedCollectionRef.current));
  }, [onFeedRefreshed, requestItems]);

  return (
    <div className={cx(className, css.ItemList)}>
      <div className={css.Header}>
        <div className={cx(css.HeadingWrapper)}>
          {!isSelectableHeader && (
            <>
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
                <Tooltip title="Edit">
                  <IconButton size="small" onClick={handleEdit}>
                    <Settings sx={{ fontSize: 16 }} />
                  </IconButton>
                </Tooltip>
              )}
            </>
          )}

          {isSelectableHeader && (
            <div className={css.HeadingControl}>
              <Tooltip title="Refresh">
                <IconButton
                  className={cx(
                    css.RefreshButton,
                    refreshSpin && css.RefreshLoading
                  )}
                  size="small"
                  onClick={handleRefresh}
                >
                  <Refresh fontSize="small" />
                </IconButton>
              </Tooltip>

              <Select
                className={css.CollectionSelect}
                size="small"
                value={selectedCollection.id}
                onChange={handleCollectionChange}
              >
                <ListSubheader>Shortcuts</ListSubheader>
                {SHORTCUTS.map((s) => (
                  <MenuItem key={s.id} value={s.id}>
                    <Typography component="span" variant="body2">
                      {s.title}
                    </Typography>
                  </MenuItem>
                ))}
                {topics.length > 0 && <ListSubheader>Topics</ListSubheader>}
                {topics.map((t) => (
                  <MenuItem key={t.id} value={t.id}>
                    <Typography component="span" variant="body2">
                      {t.title}
                    </Typography>
                  </MenuItem>
                ))}
                {feeds.length > 0 && <ListSubheader>Feeds</ListSubheader>}
                {feeds.map((f) => (
                  <MenuItem key={f.id} value={f.id}>
                    <Typography component="span" variant="body2">
                      {f.title}
                    </Typography>
                  </MenuItem>
                ))}
              </Select>
            </div>
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

        {isTopic(selectedCollection) && (
          <Select
            className={css.SortSelect}
            size="small"
            value={topicSort}
            variant="standard"
            onChange={(e) => setTopicSort(e.target.value as TopicSort)}
          >
            <MenuItem value="relevance">Most relevant</MenuItem>
            <MenuItem value="timestamp">Most recent</MenuItem>
          </Select>
        )}

        <TextField
          className={css.FilterInput}
          size="small"
          fullWidth
          placeholder="Filter…"
          value={filterText}
          onChange={(e) => setFilterText(e.target.value)}
          variant="standard"
          slotProps={{
            input: {
              endAdornment: filterText ? (
                <InputAdornment position="end">
                  <Tooltip title="Clear Filter">
                    <IconButton
                      size="small"
                      aria-label="Clear filter"
                      onClick={() => setFilterText("")}
                      edge="end"
                    >
                      <Clear fontSize="small" />
                    </IconButton>
                  </Tooltip>
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
            {getEmptyMessage(filter ?? "unread", "")}
          </Typography>
        </div>
      )}

      {items.length > 0 && displayItems.length === 0 && (
        <div className={cx(css.List, css.Placeholder)}>
          <Typography component="div" variant="body2" color="text.disabled">
            {getEmptyMessage(filter ?? "unread", filterText)}
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
                onClick={() => selectItem(item)}
              >
                <ListItemText
                  primary={
                    <Quotes
                      quoted={isAnnotation(item)}
                      color={isAnnotation(item) ? item.color : undefined}
                    >
                      {item.title}
                    </Quotes>
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
      </div>
    </div>
  );
}
