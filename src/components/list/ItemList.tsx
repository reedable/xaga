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
  useState,
  type MouseEvent,
} from "react";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import { Virtuoso } from "react-virtuoso";
import { RELATED_THRESHOLD, TOPIC_THRESHOLD } from "../../consts";
import useAbortController from "../../hooks/useAbortController";
import useEntries from "../../hooks/useEntries";
import { useEvents } from "../../hooks/useEvents";
import { useFeeds } from "../../hooks/useFeeds";
import useRelatedItems from "../../hooks/useRelatedItems";
import { useSaved } from "../../hooks/useSaved";
import {
  ALL_ITEMS,
  ANNOTATION_ITEMS,
  RECOMMENDED_ITEMS,
  SAVED_ITEMS,
  SIMILAR_TO_SAVED_ITEMS,
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
import { useManageFeed } from "../../providers/ManageFeedProvider";
import { useManageTopic } from "../../providers/ManageTopicProvider";
import cx from "../../utils/cx";
import ItemStatus from "../ItemStatus";
import Quotes from "../Quotes";
import css from "./_.module.css";

function getEmptyMessage(filter: Filter, filterText: string): string {
  const label = filter === "all" ? "items" : `${filter} items`;
  return filterText ? `No ${label} match "${filterText}"` : `No ${label}`;
}

interface ItemListProps {
  className?: string;
  selectedCollection: Collection;
}

export function ItemList({ className, selectedCollection }: ItemListProps) {
  const navigate = useNavigate();
  const { collectionId, itemId: selectedItemId } = useParams();
  const [searchParams, setSearchParams] = useSearchParams();

  const rawFilter = searchParams.get("filter");
  const filter: Filter = isFilter(rawFilter) ? rawFilter : "unread";
  const search = searchParams.get("search") ?? "";

  const { startEditingFeed } = useManageFeed();
  const { startEditingTopic } = useManageTopic();

  const [abort, abortControllerRef] = useAbortController();
  const [loading, setLoading] = useState(false);

  const { listEntries, listAllEntries, markEntryRead } =
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
  const [prevSelectedList, setPrevSelectedList] = useState(selectedCollection);

  if (prevSelectedList !== selectedCollection) {
    setPrevSelectedList(selectedCollection);
    setFilterText("");
  }

  const handleEdit = useMemo(() => {
    if (isFeed(selectedCollection)) return () => startEditingFeed(selectedCollection);
    if (isTopic(selectedCollection)) return () => startEditingTopic(selectedCollection);
    return undefined;
  }, [selectedCollection, startEditingFeed, startEditingTopic]);

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
      filter,
      listSaved,
      queryByAnnotations,
      queryBySaved,
      listEntries,
      queryByItem,
      listAllEntries,
    ]
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
    [refreshAllFeeds, refreshFeed, requestItems]
  );

  const handleClickItem = useCallback(
    (item: Item) => {
      const params = new URLSearchParams();
      params.set("filter", filter);
      if (search) params.set("search", search);
      navigate(`/${collectionId}/${item.id}?${params.toString()}`);
    },
    [navigate, collectionId, filter, search]
  );

  const handleFilter = useCallback(
    (_event: MouseEvent<HTMLElement>, value: string) => {
      if (isFilter(value)) {
        setSearchParams((prev) => { prev.set("filter", value); return prev; });
      }
    },
    [setSearchParams]
  );

  const handleRefresh = useCallback(() => {
    abort();
    (async () => { refreshList(selectedCollection); })();
  }, [abort, refreshList, selectedCollection]);

  const handleMarkEntriesRead = useCallback(async () => {
    await Promise.all(
      displayedItems
        .filter(isEntry)
        .map((entry) => entry.id)
        .map(markEntryRead)
    );
  }, [displayedItems, markEntryRead]);

  useEffect(() => {
    abort();
    (async () => { requestItems(selectedCollection); })();
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

      {items.length > 0 && displayedItems.length === 0 && (
        <div className={cx(css.List, css.Placeholder)}>
          <Typography component="div" variant="body2" color="text.disabled">
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
