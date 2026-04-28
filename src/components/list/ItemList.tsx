import {
  Clear,
  FormatQuote,
  Settings
} from "@mui/icons-material";
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
  useState,
  type MouseEvent,
} from "react";
import { Virtuoso } from "react-virtuoso";
import { RELATED_THRESHOLD, TOPIC_THRESHOLD } from "../../consts";
import useAbortController from "../../hooks/useAbortController";
import useEntries from "../../hooks/useEntries";
import { useEvents } from "../../hooks/useEvents";
import { useFeeds } from "../../hooks/useFeeds";
import useRelatedItems from "../../hooks/useRelatedItems";
import {
  ALL_ITEMS,
  ANNOTATION_ITEMS,
  RECOMMENDED_ITEMS,
} from "../../Shortcuts";
import {
  isAnnotation,
  isEntry,
  isFeed,
  isFilter,
  isTopic,
  type Collection,
  type Filter,
  type Item,
} from "../../types";

import cx from "../../utils/cx";
import ItemStatus from "../ItemStatus";
import css from "./_.module.css";

function getEmptyMessage(filter: Filter, filterText: string): string {
  const label = filter === "all" ? "items" : `${filter} items`;
  return filterText ? `No ${label} match "${filterText}"` : `No ${label}`;
}

interface ItemListProps {
  className?: string;
  selectedCollection: Collection;
  selectedFilter?: Filter;
  onSelectItem: (item: Item) => void;
  onFilterChange?: (filter: Filter) => void;
  onEdit?: () => void;
}

export function ItemList({
  className,
  selectedCollection,
  selectedFilter,
  onSelectItem,
  onFilterChange,
  onEdit,
}: ItemListProps) {
  const [abort, abortControllerRef] = useAbortController();
  const [loading, setLoading] = useState(false);

  const {
    listEntries,
    listAllEntries,
    markEntryRead,
  } = useEntries(abortControllerRef);

  const {
    queryByItem,
    queryByBookmarks,
    queryByAnnotations,
  } = useRelatedItems(abortControllerRef);

  const {
    loading: loadingFeeds,
    refreshFeed,
    refreshAllFeeds,
  } = useFeeds(abortControllerRef);

  const { onUnreadChanged, onBookmarkChanged } = useEvents();

  const [items, setItems] = useState<Item[]>([]);
  const [filter, setFilter] = useState<Filter>(selectedFilter ?? "unread");
  const [filterText, setFilterText] = useState("");
  const [prevSelectedList, setPrevSelectedList] = useState(selectedCollection);
  const [selectedItemId, setSelectedItemId] = useState<string | null>(null);

  if (prevSelectedList !== selectedCollection) {
    setPrevSelectedList(selectedCollection);
    setFilterText("");
  }

  const displayedItems = useMemo(() => {
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
          // const entries = await queryByTopics(filter, RELATED_THRESHOLD);
          const entries = await queryByBookmarks(filter, 0.82);
          if (entries) setItems(entries);
        } else if (selectedList === ANNOTATION_ITEMS) {
          const entries = await queryByAnnotations(filter, RELATED_THRESHOLD);
          if (entries) setItems(entries);
        } else if (isFeed(selectedList)) {
          const entries = await listEntries(selectedList, filter);
          if (entries) setItems(entries);
        } else if (isTopic(selectedList)) {
          const items = await queryByItem(
            selectedList,
            filter,
            selectedList.threshold ?? TOPIC_THRESHOLD,
          );
          if (items) setItems(items);
        } else {
          const entries = await listAllEntries(filter);
          if (entries) setItems(entries);
        }
      } finally {
        setLoading(false);
      }
    },
    [
      queryByBookmarks,
      queryByAnnotations,
      filter,
      listEntries,
      queryByItem,
      listAllEntries,
    ],
  );

  const refreshList = useCallback(
    async (list: Item = ALL_ITEMS) => {
      if (isFeed(list)) {
        await refreshFeed(list.id);
        await requestItems(list);
      } else {
        await refreshAllFeeds(() => requestItems(list));
      }
    },
    [refreshAllFeeds, refreshFeed, requestItems],
  );

  const handleClickItem = useCallback(
    async (item: Item) => {
      setSelectedItemId(item.id);
      onSelectItem(item);

      if (isEntry(item)) {
        await markEntryRead(item);
      }
    },
    [markEntryRead, onSelectItem],
  );

  const handleFilter = useCallback(
    (_event: MouseEvent<HTMLElement>, value: string) => {
      if (isFilter(value)) {
        setFilter(value);
        onFilterChange?.(value);
      }
    },
    [onFilterChange],
  );

  const handleRefresh = useCallback(() => {
    abort();

    (async () => {
      refreshList(selectedCollection);
    })();
  }, [abort, refreshList, selectedCollection]);

  const handleMarkEntriesRead = useCallback(async () => {
    await Promise.all(displayedItems.filter(isEntry).map(markEntryRead));
  }, [displayedItems, markEntryRead]);

  useEffect(() => {
    (async () => setFilter(selectedFilter ?? "unread"))();
  }, [selectedFilter]);

  useEffect(() => {
    abort();

    (async () => {
      requestItems(selectedCollection);
    })();
  }, [selectedCollection, requestItems, abort]);

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
          >
            {!selectedCollection ? "All items" : (selectedCollection?.title ?? "Entries")}
          </Typography>

          {onEdit && (
            <IconButton size="small" onClick={onEdit}>
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
          {/* <ToggleButton size="small" value="bookmarked" aria-label="Bookmarked">
            Bookmarked
          </ToggleButton> */}
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
          <LinearProgress variant="indeterminate" color={loadingFeeds ? 'warning' : 'primary'} />
        </Collapse>
      </div>

      {!loading && items.length === 0 && (
        <div className={cx(css.List, css.Placeholder)}>
          <Typography variant="body2" color="text.disabled">
            {getEmptyMessage(filter, "")}
          </Typography>
        </div>
      )}

      {items.length > 0 && displayedItems.length === 0 && (
        <div className={cx(css.List, css.Placeholder)}>
          <Typography variant="body2" color="text.disabled">
            {getEmptyMessage(filter, filterText)}
          </Typography>
        </div>
      )}

      {displayedItems.length > 0 && (
        <Virtuoso
          className={css.List}
          data={displayedItems}
          itemContent={(_index, item) => (
            <div key={item.id} className={css.ListItem}>
              <ListItemButton
                className={css.ListItemButton}
                selected={selectedItemId === item.id}
                onClick={() => handleClickItem(item)}
              >
                <ListItemText
                  primary={
                    <>
                      {isAnnotation(item) && (
                        <FormatQuote sx={{ transform: "rotate(180deg)" }} />
                      )}
                      {item.title}
                      {isAnnotation(item) && <FormatQuote />}
                    </>
                  }
                  secondary={
                    <ItemStatus item={item} />
                  }
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
          <Typography variant="body2" color="text.disabled">
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
