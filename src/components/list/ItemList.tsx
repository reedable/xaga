import {
  Bookmark,
  Circle,
  Clear,
  FormatQuote,
  Settings,
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
import { useBookmarks } from "../../hooks/useBookmarks";
import useEntries from "../../hooks/useEntries";
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
  isItem,
  isTopic,
  type Filter,
  type Item,
} from "../../types";

import cx from "../../utils/cx";
import formatByline from "../../utils/formatByline";
import css from "./_.module.css";

function getEmptyMessage(filter: Filter, filterText: string): string {
  const label = filter === "all" ? "items" : `${filter} items`;
  return filterText ? `No ${label} match "${filterText}"` : `No ${label}`;
}

interface ItemListProps {
  className?: string;
  selectedList: Item;
  selectedFilter?: Filter;
  onSelectItem: (item: Item) => void;
  onEditFeed?: () => void;
}

export function ItemList({
  className,
  selectedList,
  selectedFilter,
  onSelectItem,
  onEditFeed,
}: ItemListProps) {
  const [abort, abortControllerRef] = useAbortController();

  const {
    loading: loadingEntries,
    listEntries,
    listAllEntries,
    markEntryRead,
  } = useEntries(abortControllerRef);

  const {
    loading: loadingRelatedItems,
    queryByItem,
    queryByBookmarks,
    queryByAnnotations,
  } = useRelatedItems(abortControllerRef);

  const {
    loading: loadingFeeds,
    refreshFeed,
    refreshAllFeeds,
  } = useFeeds(abortControllerRef);

  const { bookmarkMap } = useBookmarks(abortControllerRef);

  const [items, setItems] = useState<Item[]>([]);
  const [filter, setFilter] = useState<Filter>(selectedFilter ?? "unread");
  const [filterText, setFilterText] = useState("");
  const [prevSelectedList, setPrevSelectedList] = useState(selectedList);
  const [selectedItemId, setSelectedItemId] = useState<string | null>(null);

  if (prevSelectedList !== selectedList) {
    setPrevSelectedList(selectedList);
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
        const i = await markEntryRead(item);
        if (i) setItems((prev) => prev.map((p) => (p.id === i.id ? i : p)));
      }
    },
    [markEntryRead, onSelectItem],
  );

  const handleFilter = useCallback(
    (_event: MouseEvent<HTMLElement>, value: string) => {
      if (isFilter(value)) {
        setFilter(value);
      }
    },
    [],
  );

  const handleRefresh = useCallback(() => {
    abort();

    (async () => {
      refreshList(selectedList);
    })();
  }, [abort, refreshList, selectedList]);

  const handleMarkEntriesRead = useCallback(async () => {
    const promises = displayedItems.filter(isEntry).map(markEntryRead);

    promises.map(async (promise) => {
      const item = await promise;
      if (isItem(item)) {
        setItems((prev) => prev.map((p) => (p.id === item.id ? item : p)));
      }
    });

    await Promise.all(promises);
  }, [displayedItems, markEntryRead]);

  useEffect(() => {
    (async () => setFilter(selectedFilter ?? "all"))();
  }, [selectedFilter]);

  useEffect(() => {
    abort();

    (async () => {
      requestItems(selectedList);
    })();
  }, [selectedList, requestItems, abort]);

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
            {!selectedList ? "All items" : (selectedList?.title ?? "Entries")}
          </Typography>

          {onEditFeed && (
            <IconButton size="small" onClick={onEditFeed}>
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
        <Collapse in={loadingEntries || loadingRelatedItems || loadingFeeds}>
          <LinearProgress variant="indeterminate" />
        </Collapse>
      </div>

      {!loadingEntries && !loadingRelatedItems && items.length === 0 && (
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
                    <>
                      {isEntry(item) && item.isUnread && (
                        <Circle className={css.UnreadIcon} color="primary" />
                      )}

                      {bookmarkMap?.get(item.id) && (
                        <Bookmark color="primary" />
                      )}

                      <span>{formatByline(item)}</span>
                    </>
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

      {(loadingEntries || loadingRelatedItems) && items.length === 0 && (
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
