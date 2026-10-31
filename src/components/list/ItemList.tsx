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
import { Virtuoso, type VirtuosoHandle } from "react-virtuoso";
import { usePlatform } from "../../platform/Platform";
import { useErrorSink } from "../../hooks/useErrorSink";
import { useMissingPins } from "../../hooks/useMissingPins";
import MissingShortcutsButton from "../sidebar/MissingShortcutsButton";
import { usePinnedEntries } from "../../hooks/usePinnedEntries";
import { articleScopeItems } from "../../utils/articleScopeItems";
import useAbortController from "../../hooks/useAbortController";
import useEntries from "../../hooks/useEntries";
import { useEvents } from "../../hooks/useEvents";
import { useFeeds } from "../../hooks/useFeeds";
import useRouter from "../../hooks/useRouter";
import { useSites } from "../../hooks/useSites";
import { useTopics } from "../../hooks/useTopics";
import { onTopicItemAdded } from "../../events/topicMembership";
import { ALL_ITEMS, SHORTCUTS } from "../../Shortcuts";
import {
  isPartialAnnotation,
  isEntry,
  isFeed,
  isTopic,
  type Entry,
  type Feed,
  type Filter,
  type Item,
  type Topic,
  type Site,
  isSite,
  type TopicSort,
} from "../../types";
import cx from "../../utils/cx";
import sortByTitle from "../../utils/sortByTitle";
import {
  itemMatchesTextFilter,
  mergeAllItemsAnnotationChange,
} from "../../utils/mergeAllItemsAnnotationChange";
import { mergeTopicAddition } from "../../utils/mergeTopicAddition";
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
  const {
    filter,
    selectItem,
    selectSite,
    selectArticleScope,
    selectAnnotation,
    state,
    selectFilter,
  } = useRouter();
  const platform = usePlatform();
  const { pushError } = useErrorSink();
  const pinnedEntries = usePinnedEntries();
  const { articleScopeId } = state;
  const selectedItemId = state.annotationId ?? state.entryId;
  const [scopedArticle, setScopedArticle] = useState<Entry | null>(null);
  const articleRequestRef = useRef(0);
  const articleStateRef = useRef<{ version: number; entry?: Entry }>({
    version: 0,
  });
  const articleScopeRef = useRef(articleScopeId);
  articleScopeRef.current = articleScopeId;
  const { feedId, topicId, shortcutId } = state;
  const { siteHostname } = state;
  const [abort, abortControllerRef] = useAbortController();
  const { topics, listTopicItems } = useTopics(abortControllerRef);
  const { sites, listSiteEntries } = useSites(abortControllerRef);

  const {
    feeds,
    loading: loadingFeeds,
    refreshAllFeeds,
  } = useFeeds(abortControllerRef);

  const missingPins = useMissingPins(pinnedEntries, feeds, topics);
  const [loading, setLoading] = useState(false);
  const [refreshSpin, setRefreshSpin] = useState(false);

  const selectedCollection = useMemo(
    () =>
      (articleScopeId &&
        (scopedArticle?.id === articleScopeId
          ? scopedArticle
          : pinnedEntries.find((entry) => entry.id === articleScopeId))) ||
      (feedId && feeds.find((f) => f.id === feedId)) ||
      (topicId && topics.find((t) => t.id === topicId)) ||
      (siteHostname && sites.find((site) => site.hostname === siteHostname)) ||
      (shortcutId && SHORTCUTS.find((s) => s.id === shortcutId)) ||
      ALL_ITEMS,
    [
      articleScopeId,
      scopedArticle,
      pinnedEntries,
      feedId,
      topicId,
      siteHostname,
      shortcutId,
      feeds,
      topics,
      sites,
    ]
  );

  const articleOptions = useMemo(
    () =>
      [
        ...pinnedEntries,
        ...(articleScopeId &&
        scopedArticle?.id === articleScopeId &&
        !pinnedEntries.some((entry) => entry.id === articleScopeId)
          ? [scopedArticle]
          : []),
      ].sort(sortByTitle),
    [pinnedEntries, articleScopeId, scopedArticle]
  );

  const { listEntries, listAllItems, markEntryRead } =
    useEntries(abortControllerRef);

  const {
    onFeedsRefreshed,
    onUnreadChanged,
    onBookmarkChanged,
    onAnnotationChanged,
    onAnnotationReplaced,
    onAnnotationRemoved,
    onEmbeddingsChanged,
  } = useEvents();

  const [items, setItems] = useState<Item[]>([]);
  const [filterText, setFilterText] = useState("");
  const [topicSort, setTopicSort] = useState<TopicSort>("relevance");
  const allItemsMergeRef = useRef({ shortcutId, filter, filterText });
  allItemsMergeRef.current = { shortcutId, filter, filterText };

  // Virtuoso only renders on-screen rows, so moving the selection off-screen
  // requires an imperative scroll to bring the target row into view before it
  // can receive DOM focus. itemRefs holds the focusable node of each rendered
  // row, keyed by item id.
  const virtuosoRef = useRef<VirtuosoHandle>(null);
  const itemRefs = useRef(new Map<string, HTMLDivElement>());

  const handleEdit = useMemo(() => {
    if (isFeed(selectedCollection)) return () => onEditFeed(selectedCollection);
    if (isTopic(selectedCollection))
      return () => onEditTopic(selectedCollection);
    return undefined;
  }, [selectedCollection, onEditFeed, onEditTopic]);

  const handleCollectionChange = useCallback(
    (event: SelectChangeEvent<string>) => {
      const id = event.target.value;
      const article = pinnedEntries.find(
        (entry) => `article:${entry.id}` === id
      );
      if (article) {
        selectArticleScope(article);
        return;
      }
      const collection = [...SHORTCUTS, ...topics, ...feeds].find(
        (item) => item.id === id
      );
      if (collection) selectItem(collection);
      const site = sites.find((item) => item.hostname === id);
      if (site) selectSite(site);
    },
    [
      feeds,
      selectItem,
      selectSite,
      sites,
      topics,
      pinnedEntries,
      selectArticleScope,
    ]
  );

  const handleRefresh = useCallback(() => {
    refreshAllFeeds();
    setRefreshSpin(true);
  }, [refreshAllFeeds]);

  useEffect(() => {
    if (!loadingFeeds) setRefreshSpin(false);
  }, [loadingFeeds]);

  const displayItems = useMemo(() => {
    return items.filter((item) => itemMatchesTextFilter(item, filterText));
  }, [items, filterText]);

  // Up/Down move the selection to the adjacent visible item, clamped at both
  // ends (no wrap). Selecting reuses selectItem, so it loads the item's content
  // and pushes history exactly like a click; the newly selected row is then
  // scrolled into view and given DOM focus so keyboard operation can continue
  // from there (WCAG 2.1.1).
  const handleItemKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
      if (e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return;

      const currentIndex = displayItems.findIndex(
        (i) => itemRefs.current.get(i.id) === e.currentTarget
      );
      if (currentIndex === -1) return;

      const nextIndex = currentIndex + (e.key === "ArrowDown" ? 1 : -1);
      if (nextIndex < 0 || nextIndex >= displayItems.length) return;

      e.preventDefault();
      const next = displayItems[nextIndex];
      selectItem(next);
      virtuosoRef.current?.scrollIntoView({
        index: nextIndex,
        behavior: "auto",
        done: () => itemRefs.current.get(next.id)?.focus(),
      });
    },
    [displayItems, selectItem]
  );

  const requestArticleScope = useCallback(async () => {
    if (!articleScopeId) return;
    const version = ++articleRequestRef.current;
    const stateVersion = articleStateRef.current.version;
    const signal = abortControllerRef.current.signal;
    setLoading(true);
    try {
      const article = await platform.readArticleScope(signal, articleScopeId);
      const next = await articleScopeItems(article, (id, refs) =>
        platform.readEntryImages(signal, id, refs)
      );
      if (
        signal.aborted ||
        version !== articleRequestRef.current ||
        articleScopeRef.current !== articleScopeId
      )
        return;
      // State events can arrive while rendering/ordering a file-read snapshot.
      const latest = articleStateRef.current;
      if (latest.version > stateVersion && latest.entry?.id === article.id) {
        article.isUnread = latest.entry.isUnread;
        article.color = latest.entry.color;
      }
      setScopedArticle(article);
      setItems(next);
    } catch (error) {
      if (!signal.aborted && articleScopeRef.current === articleScopeId)
        pushError(error);
    } finally {
      if (
        !signal.aborted &&
        version === articleRequestRef.current &&
        articleScopeRef.current === articleScopeId
      )
        setLoading(false);
    }
  }, [articleScopeId, abortControllerRef, platform, pushError]);

  const requestItems = useCallback(
    async (selectedList: Item | Site) => {
      if (articleScopeId) {
        await requestArticleScope();
        return;
      }
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
        } else if (isSite(selectedList)) {
          const entries = await listSiteEntries(selectedList.hostname, filter);
          if (entries) setItems(entries);
        } else {
          const entries = await listAllItems(filter);
          if (entries) setItems(entries);
        }
      } finally {
        setLoading(false);
      }
    },
    [
      articleScopeId,
      requestArticleScope,
      filter,
      listEntries,
      listTopicItems,
      listSiteEntries,
      listAllItems,
      topicSort,
    ]
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

  const mergeAddedTopicItem = useCallback(
    async (changedTopicId: string, itemId: string) => {
      if (changedTopicId !== topicId) return;
      const authoritative = await listTopicItems(
        changedTopicId,
        filter ?? "unread",
        topicSort
      );
      if (
        !authoritative ||
        !isTopic(selectedCollectionRef.current) ||
        selectedCollectionRef.current.id !== changedTopicId
      )
        return;
      setItems((current) => mergeTopicAddition(current, authoritative, itemId));
    },
    [topicId, filter, topicSort, listTopicItems]
  );

  useEffect(() => {
    abort();
    (async () => {
      requestItems(selectedCollectionRef.current);
    })();
    // Depend on IDs, not objects, so feed/topic identity changes from
    // requestFeeds/requestTopics don't re-trigger a full list reload.
  }, [
    articleScopeId,
    feedId,
    topicId,
    siteHostname,
    shortcutId,
    requestItems,
    abort,
  ]);

  useEffect(() => {
    return onEmbeddingsChanged(() => {
      if (siteHostname) void requestItems(selectedCollectionRef.current);
    });
  }, [onEmbeddingsChanged, requestItems, siteHostname]);

  useEffect(() => {
    return onUnreadChanged((entry) => {
      if (entry.id === articleScopeRef.current) {
        articleStateRef.current = {
          version: articleStateRef.current.version + 1,
          entry,
        };
      }
      setItems((prev) => prev.map((p) => (p.id === entry.id ? entry : p)));
    });
  }, [onUnreadChanged]);

  useEffect(() => {
    return onBookmarkChanged((entry) => {
      if (entry.id === articleScopeRef.current) {
        articleStateRef.current = {
          version: articleStateRef.current.version + 1,
          entry,
        };
      }
      setItems((prev) => prev.map((p) => (p.id === entry.id ? entry : p)));
    });
  }, [onBookmarkChanged]);

  useEffect(() => {
    return onAnnotationChanged((item) => {
      if (
        articleScopeId &&
        "reference" in item &&
        item.reference === articleScopeId
      ) {
        void requestArticleScope();
        return;
      }
      const active = allItemsMergeRef.current;
      setItems((current) =>
        mergeAllItemsAnnotationChange(
          current,
          item,
          active.shortcutId === ALL_ITEMS.id,
          active.filter,
          active.filterText
        )
      );
      if (topicId) void mergeAddedTopicItem(topicId, item.id);
    });
  }, [
    onAnnotationChanged,
    topicId,
    mergeAddedTopicItem,
    articleScopeId,
    requestArticleScope,
  ]);

  useEffect(() => {
    return onAnnotationReplaced(({ oldId, item }) => {
      if (
        articleScopeId &&
        "reference" in item &&
        item.reference === articleScopeId
      ) {
        if (state.annotationId === oldId && isPartialAnnotation(item))
          selectAnnotation(item);
        void requestArticleScope();
        return;
      }
      setItems((prev) =>
        prev.map((value) => (value.id === oldId ? item : value))
      );
    });
  }, [
    onAnnotationReplaced,
    articleScopeId,
    requestArticleScope,
    state.annotationId,
    selectAnnotation,
  ]);

  useEffect(() => {
    return onAnnotationRemoved((id) => {
      if (articleScopeId) {
        void requestArticleScope();
        return;
      }
      setItems((prev) => prev.filter((p) => p.id !== id));
    });
  }, [onAnnotationRemoved, articleScopeId, requestArticleScope]);

  useEffect(() => {
    return onFeedsRefreshed(() => requestItems(selectedCollectionRef.current));
  }, [onFeedsRefreshed, requestItems]);

  useEffect(
    () =>
      onTopicItemAdded(async ({ topicId: changedTopicId, itemId }) => {
        await mergeAddedTopicItem(changedTopicId, itemId);
      }),
    [mergeAddedTopicItem]
  );

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
                value={
                  articleScopeId
                    ? `article:${articleScopeId}`
                    : selectedCollection.id
                }
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
                {articleScopeId &&
                  !articleOptions.some(
                    (entry) => entry.id === articleScopeId
                  ) && (
                    <MenuItem value={`article:${articleScopeId}`} disabled>
                      Loading article…
                    </MenuItem>
                  )}
                {articleOptions.map((entry) => (
                  <MenuItem
                    key={`article:${entry.id}`}
                    value={`article:${entry.id}`}
                  >
                    <Typography component="span" variant="body2">
                      {entry.title}
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
                {sites.length > 0 && <ListSubheader>Sites</ListSubheader>}
                {sites.map((site) => (
                  <MenuItem key={site.hostname} value={site.hostname}>
                    <Typography component="span" variant="body2">
                      {site.title}
                    </Typography>
                  </MenuItem>
                ))}
              </Select>
              <MissingShortcutsButton targets={missingPins} />
            </div>
          )}
        </div>

        {!articleScopeId && (
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
        )}

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
            {getEmptyMessage(articleScopeId ? "all" : (filter ?? "unread"), "")}
          </Typography>
        </div>
      )}

      {items.length > 0 && displayItems.length === 0 && (
        <div className={cx(css.List, css.Placeholder)}>
          <Typography component="div" variant="body2" color="text.disabled">
            {getEmptyMessage(
              articleScopeId ? "all" : (filter ?? "unread"),
              filterText
            )}
          </Typography>
        </div>
      )}

      {displayItems.length > 0 && (
        <Virtuoso
          ref={virtuosoRef}
          className={css.List}
          data={displayItems}
          itemContent={(_index, item) => (
            <div key={item.id} className={css.ListItem}>
              <ListItemButton
                ref={(node: HTMLDivElement | null) => {
                  if (node) itemRefs.current.set(item.id, node);
                  else itemRefs.current.delete(item.id);
                }}
                className={css.ListItemButton}
                selected={selectedItemId === item.id}
                tabIndex={
                  selectedItemId === item.id ||
                  (!displayItems.some((value) => value.id === selectedItemId) &&
                    displayItems[0].id === item.id)
                    ? 0
                    : -1
                }
                onKeyDown={handleItemKeyDown}
                onClick={() => selectItem(item)}
              >
                <ListItemText
                  primary={
                    <Quotes
                      quoted={isPartialAnnotation(item)}
                      color={isPartialAnnotation(item) ? item.color : undefined}
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

      {!articleScopeId && (
        <div className={css.Toolbar}>
          <Button onClick={handleMarkEntriesRead}>Mark all read</Button>
        </div>
      )}
    </div>
  );
}
