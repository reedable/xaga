import { Bookmark, Circle } from "@mui/icons-material";
import {
  Button,
  Collapse,
  LinearProgress,
  ListItemButton,
  ListItemText,
  ToggleButton,
  ToggleButtonGroup,
  Typography,
} from "@mui/material";
import { useCallback, useEffect, useState, type MouseEvent } from "react";
import { Virtuoso } from "react-virtuoso";
import { RELATED_THRESHOLD, TOPIC_THRESHOLD } from "../../consts";
import useAbortController from "../../hooks/useAbortController";
import { useBookmarks } from "../../hooks/useBookmarks";
import useEntries from "../../hooks/useEntries";
import { useFeeds } from "../../hooks/useFeeds";
import useRelatedItems from "../../hooks/useRelatedItems";
import { ALL_ITEMS, RECOMMENDED_ITEMS } from "../../Items";
import {
  isEntry,
  isFeed,
  isFilter,
  isItem,
  isTopic,
  type Entry,
  type Filter,
  type Item,
} from "../../types";
import cx from "../../utils/cx";
import formatByline from "../../utils/formatByline";
import css from "./_.module.css";

interface ItemListProps {
  className?: string;
  selectedList: Item;
  selectedFilter?: Filter;
  onSelectArticle: (entry: Entry) => void;
}

export function ItemList({
  className,
  selectedList,
  selectedFilter,
  onSelectArticle,
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
  } = useRelatedItems(abortControllerRef);

  const {
    loading: loadingFeeds,
    refreshFeed,
    refreshAllFeeds,
  } = useFeeds(abortControllerRef);

  const { bookmarkMap } = useBookmarks(abortControllerRef);

  const [items, setItems] = useState<Item[]>([]);
  const [filter, setFilter] = useState<Filter>(selectedFilter ?? "all");
  const [selectedItemId, setSelectedItemId] = useState<string | null>(null);

  const requestItems = useCallback(
    async (selectedList: Item) => {
      if (selectedList === RECOMMENDED_ITEMS) {
        // const entries = await queryByTopics(filter, RELATED_THRESHOLD);
        const entries = await queryByBookmarks(filter, RELATED_THRESHOLD);
        if (entries) setItems(entries);
      } else if (isFeed(selectedList)) {
        const entries = await listEntries(selectedList, filter);
        if (entries) setItems(entries);
      } else if (isTopic(selectedList)) {
        const items = await queryByItem(selectedList, filter, TOPIC_THRESHOLD);
        if (items) setItems(items);
      } else {
        const entries = await listAllEntries(filter);
        if (entries) setItems(entries);
      }
    },
    [queryByBookmarks, filter, listEntries, queryByItem, listAllEntries],
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

      if (isEntry(item)) {
        onSelectArticle(item);
        const e = await markEntryRead(item);
        if (e) setItems((prev) => prev.map((p) => (p.id === e.id ? e : p)));
      }
    },
    [markEntryRead, onSelectArticle],
  );

  const handleFilter = useCallback(
    (_event: MouseEvent<HTMLElement>, value: string) => {
      if (isFilter(value)) {
        setFilter(value);
      } else {
        setFilter("all");
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
    const promises = items.filter(isEntry).map(markEntryRead);

    promises.map(async (promise) => {
      const item = await promise;
      if (isItem(item)) {
        setItems((prev) => prev.map((p) => (p.id === item.id ? item : p)));
      }
    });

    await Promise.all(promises);
    await refreshList(selectedList);
  }, [items, markEntryRead, refreshList, selectedList]);

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
        <Typography variant="overline" color="text.secondary">
          {!selectedList ? "All items" : (selectedList?.title ?? "Entries")}
        </Typography>

        <ToggleButtonGroup
          aria-label="Filter"
          className={css.FilterButtonGroup}
          exclusive
          value={filter}
          onChange={handleFilter}
        >
          <ToggleButton size="small" value="all" aria-label="All">
            All
          </ToggleButton>
          <ToggleButton size="small" value="unread" aria-label="Unread">
            Unread
          </ToggleButton>
          <ToggleButton size="small" value="bookmarked" aria-label="Bookmarked">
            Bookmarked
          </ToggleButton>
          <ToggleButton size="small" value="upcoming" aria-label="Upcoming">
            Upcoming
          </ToggleButton>
        </ToggleButtonGroup>
      </div>

      <div>
        <Collapse in={loadingEntries || loadingRelatedItems || loadingFeeds}>
          <LinearProgress variant="indeterminate" />
        </Collapse>
      </div>

      {!loadingEntries && !loadingRelatedItems && items.length === 0 && (
        <div className={cx(css.List, css.Placeholder)}>
          <Typography variant="body2" color="text.disabled">
            No entries
          </Typography>
        </div>
      )}

      {items.length > 0 && (
        <Virtuoso
          className={css.List}
          data={items}
          itemContent={(_index, item) => (
            <div key={item.id} className={css.ListItem}>
              <ListItemButton
                className={css.ListItemButton}
                selected={selectedItemId === item.id}
                onClick={() => handleClickItem(item)}
              >
                <ListItemText
                  primary={item.title}
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
