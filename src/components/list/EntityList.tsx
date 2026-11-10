import { useShortcutTargets } from "../../hooks/useShortcutTargets";
import { pinKey } from "../../pins";
import {
  shortcutReference,
  type ShortcutNode,
} from "../../utils/projectPinTree";
import { useLibraryReadOnly } from "../../hooks/useLibraryReadOnly";
import { Clear, Refresh, Settings } from "@mui/icons-material";
import {
  Button,
  Checkbox,
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
import { useLocation, useNavigationType } from "react-router-dom";
import { onTopicEntityAdded } from "../../events/topicMembership";
import useAbortController from "../../hooks/useAbortController";
import useEntries from "../../hooks/useEntries";
import { useErrorSink } from "../../hooks/useErrorSink";
import { useEvents } from "../../hooks/useEvents";
import { useFeeds } from "../../hooks/useFeeds";
import { useMissingPins } from "../../hooks/useMissingPins";
import { usePinnedEntries } from "../../hooks/usePinnedEntries";
import useRouter from "../../hooks/useRouter";
import { useSites } from "../../hooks/useSites";
import { useTopics } from "../../hooks/useTopics";
import { usePlatform } from "../../platform/Platform";
import { useTags } from "../../providers/TagsContext";
import { ALL_ENTITIES, SHORTCUTS } from "../../Shortcuts";
import { emptySite } from "../../sites";
import {
  isEntry,
  isFilter,
  isFeed,
  isManagedCollection,
  isPartialAnnotation,
  isSite,
  isTopic,
  type Entry,
  type Feed,
  type Filter,
  type ManagedEntity,
  type Site,
  type Topic,
  type TopicSort,
} from "../../types";
import { articleScopeEntities } from "../../utils/articleScopeEntities";
import cx from "../../utils/cx";
import {
  entityMatchesTextFilter,
  mergeAllEntitiesAnnotationChange,
} from "../../utils/mergeAllEntitiesAnnotationChange";
import { mergeTopicAddition } from "../../utils/mergeTopicAddition";
import sortByTitle from "../../utils/sortByTitle";
import EntityStatus from "../EntityStatus";
import Quotes from "../Quotes";
import MissingShortcutsButton from "../sidebar/MissingShortcutsButton";
import { TagActions } from "../TagActions";
import css from "./EntityList.module.css";
import colorCss from "../../colors.module.css";

function getEmptyMessage(filter: Filter, filterText: string): string {
  const label = filter === "all" ? "items" : `${filter} items`;
  return filterText ? `No ${label} match "${filterText}"` : `No ${label}`;
}

type EntityListProps = {
  filter: Filter;
  onFilterChange: (filter: Filter) => void;
  className?: string;
  isSelectableHeader: boolean;
  onEditFeed: (feed: Feed) => void;
  onEditTopic: (topic: Topic) => void;
  onEditSite?: (site: Site) => void;
};

export function EntityList({
  filter: listFilter,
  onFilterChange,
  className,
  isSelectableHeader,
  onEditFeed,
  onEditTopic,
  onEditSite,
}: EntityListProps) {
  const readOnly = useLibraryReadOnly();
  const location = useLocation();
  const navigationType = useNavigationType();
  const lastLocationKey = useRef(location.key);
  const pendingHistorySelection = useRef<string | undefined>(undefined);
  // Request effects run before the history effect, ahead of the loading render.
  const listLoading = useRef(false);
  const {
    selectEntity,
    selectCollection,
    selectShortcut,
    selectShortcutTarget,
    selectArticleScope,
    replaceAnnotationSelection,
    state,
    selectTags,
  } = useRouter();
  const tagSelectionKey = JSON.stringify(state.tagIds);
  const tagIds: string[] | undefined = useMemo(
    () =>
      tagSelectionKey === undefined ? undefined : JSON.parse(tagSelectionKey),
    [tagSelectionKey]
  );
  const hasSelectedTags = !!tagIds?.length;
  const filter =
    hasSelectedTags && (!listFilter || listFilter === "unread")
      ? "all"
      : listFilter;
  const { tags, renaming = [], deleting = [], merging = [] } = useTags();
  const tagOptions = useMemo(
    () =>
      [
        ...tags,
        ...(tagIds ?? [])
          .filter((id) => !tags.some((tag) => tag.id === id))
          .map((id) => ({ id, name: "Unavailable tag" })),
      ].sort((a, b) => a.name.localeCompare(b.name)),
    [tags, tagIds]
  );
  const platform = usePlatform();
  const { pushError } = useErrorSink();
  const pinnedEntries = usePinnedEntries();
  const { hierarchy: shortcutHierarchy, targets: shortcutTargets } =
    useShortcutTargets();
  const { articleScopeId } = state;
  const selectedEntityId = state.annotationId ?? state.entryId;
  const [scopedArticle, setScopedArticle] = useState<Entry | null>(null);
  const articleRequestRef = useRef(0);
  const listRequestRef = useRef(0);
  const articleStateRef = useRef<{ version: number; entry?: Entry }>({
    version: 0,
  });
  const articleScopeRef = useRef(articleScopeId);
  articleScopeRef.current = articleScopeId;
  const { feedId, topicId, shortcutId } = state;
  const { siteHostname } = state;
  const [abort, abortControllerRef] = useAbortController();
  const { topics, listTopicEntities } = useTopics(abortControllerRef);
  const {
    sites,
    pinnedSites = [],
    listSiteEntries,
  } = useSites(abortControllerRef);

  const {
    feeds,
    loading: loadingFeeds,
    refreshAllFeeds,
  } = useFeeds(abortControllerRef);

  const missingPins = useMissingPins(pinnedEntries, feeds, topics);
  const [loading, setLoading] = useState(false);
  const [refreshSpin, setRefreshSpin] = useState(false);
  const [headerOpen, setHeaderOpen] = useState(false);

  const selectedScope = useMemo(
    () =>
      (tagIds !== undefined && {
        id: "__TAGS__",
        title: tagIds.length
          ? tagIds
              .map(
                (id) =>
                  tagOptions.find((tag) => tag.id === id)?.name ??
                  "Unavailable tag"
              )
              .join(" + ")
          : "All items", // TODO Make annotations taggable
        createdAt: new Date(0).toISOString(),
      }) ||
      (articleScopeId &&
        (scopedArticle?.id === articleScopeId
          ? scopedArticle
          : pinnedEntries.find((entry) => entry.id === articleScopeId))) ||
      (feedId && feeds.find((f) => f.id === feedId)) ||
      (topicId && topics.find((t) => t.id === topicId)) ||
      (siteHostname &&
        ([...sites, ...pinnedSites].find((site) => site.id === siteHostname) ??
          emptySite(siteHostname))) ||
      (shortcutId && SHORTCUTS.find((s) => s.id === shortcutId)) ||
      ALL_ENTITIES,
    [
      tagIds,
      tagOptions,
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
      pinnedSites,
    ]
  );

  const siteOptions = useMemo(() => {
    const options = new Map(
      [...sites, ...pinnedSites].map((site) => [site.id, site])
    );
    if (isSite(selectedScope)) options.set(selectedScope.id, selectedScope);
    return [...options.values()];
  }, [sites, pinnedSites, selectedScope]);

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

  const { listEntries, listAllEntities, markEntryRead } =
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

  const [entities, setEntities] = useState<ManagedEntity[]>([]);
  const [filterText, setFilterText] = useState("");
  const [topicSort, setTopicSort] = useState<TopicSort>("relevance");
  const allEntitiesMergeRef = useRef({ shortcutId, filter, filterText });
  allEntitiesMergeRef.current = { shortcutId, filter, filterText };

  // Virtuoso only renders on-screen rows, so moving the selection off-screen
  // requires an imperative scroll to bring the target row into view before it
  // can receive DOM focus. itemRefs holds the focusable node of each rendered
  // row, keyed by entity id.
  const virtuosoRef = useRef<VirtuosoHandle>(null);
  const itemRefs = useRef(new Map<string, HTMLDivElement>());

  const handleEdit = useMemo(() => {
    if (isSite(selectedScope)) return () => onEditSite?.(selectedScope);
    if (!isManagedCollection(selectedScope)) return undefined;
    if (isFeed(selectedScope)) return () => onEditFeed(selectedScope);
    if (isTopic(selectedScope)) return () => onEditTopic(selectedScope);
    return undefined;
  }, [selectedScope, onEditFeed, onEditTopic, onEditSite]);

  const activePinKey = articleScopeId
    ? `pin:entry:${articleScopeId}`
    : feedId
      ? `pin:feed:${feedId}`
      : topicId
        ? `pin:topic:${topicId}`
        : siteHostname
          ? `pin:site:${siteHostname}`
          : undefined;
  const scopeValues = [
    articleScopeId ? `article:${articleScopeId}` : selectedScope.id,
  ];
  if (
    activePinKey &&
    shortcutTargets.some(
      (target) => `pin:${pinKey(shortcutReference(target))}` === activePinKey
    )
  ) {
    scopeValues.push(activePinKey);
  }
  const headerValues =
    tagIds !== undefined
      ? tagIds.length
        ? tagIds.map((id) => `tag:${id}`)
        : ["tags:all"]
      : scopeValues;
  const handleCollectionChange = (event: SelectChangeEvent<string[]>) => {
    const values =
      typeof event.target.value === "string"
        ? event.target.value.split(",")
        : event.target.value;
    const id =
      values.find((value) => !headerValues.includes(value)) ??
      headerValues.find((value) => !values.includes(value));
    if (!id) return;
    if (id === "tags:all") {
      selectTags([]);
      return;
    }
    if (id.startsWith("tag:")) {
      selectTags(
        values
          .filter((value) => value.startsWith("tag:"))
          .map((value) => value.slice(4))
      );
      return;
    }
    setHeaderOpen(false);
    if (id.startsWith("pin:")) {
      const target = shortcutTargets.find(
        (target) => `pin:${pinKey(shortcutReference(target))}` === id
      );
      if (target) selectShortcutTarget(target);
      return;
    }
    const article = pinnedEntries.find((entry) => `article:${entry.id}` === id);
    if (article) {
      selectArticleScope(article);
      return;
    }
    const shortcut = SHORTCUTS.find((entity) => entity.id === id);
    if (shortcut) {
      selectShortcut(shortcut);
      return;
    }
    const collection = [...topics, ...feeds, ...siteOptions].find(
      (entity) => entity.id === id
    );
    if (collection) selectCollection(collection);
  };

  const displayEntities = useMemo(() => {
    return entities.filter((entity) =>
      entityMatchesTextFilter(entity, filterText)
    );
  }, [entities, filterText]);

  // Up/Down move the selection to the adjacent visible entity, clamped at both
  // ends (no wrap). Selecting reuses selectEntity, so it loads the entity's content
  // and pushes history exactly like a click; the newly selected row is then
  // scrolled into view and given DOM focus so keyboard operation can continue
  // from there (WCAG 2.1.1).
  const handleEntityKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
      if (e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return;

      const currentIndex = displayEntities.findIndex(
        (i) => itemRefs.current.get(i.id) === e.currentTarget
      );
      if (currentIndex === -1) return;

      const nextIndex = currentIndex + (e.key === "ArrowDown" ? 1 : -1);
      if (nextIndex < 0 || nextIndex >= displayEntities.length) return;

      e.preventDefault();
      const next = displayEntities[nextIndex];
      selectEntity(next);
      virtuosoRef.current?.scrollIntoView({
        index: nextIndex,
        behavior: "auto",
        done: () => itemRefs.current.get(next.id)?.focus(),
      });
    },
    [displayEntities, selectEntity]
  );

  const requestArticleScope = useCallback(async () => {
    if (!articleScopeId) return;
    const version = ++articleRequestRef.current;
    const stateVersion = articleStateRef.current.version;
    const signal = abortControllerRef.current.signal;
    setLoading(true);
    listLoading.current = true;
    try {
      const article = await platform.readArticleScope(signal, articleScopeId);
      const next = await articleScopeEntities(article, (id, refs) =>
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
      setEntities(next);
    } catch (error) {
      if (!signal.aborted && articleScopeRef.current === articleScopeId)
        pushError(error);
    } finally {
      if (
        !signal.aborted &&
        version === articleRequestRef.current &&
        articleScopeRef.current === articleScopeId
      ) {
        listLoading.current = false;
        setLoading(false);
      }
    }
  }, [articleScopeId, abortControllerRef, platform, pushError]);

  const requestEntities = useCallback(
    async (selectedList: ManagedEntity | Site) => {
      if (articleScopeId) {
        await requestArticleScope();
        return;
      }
      const signal = abortControllerRef.current.signal;
      const request = ++listRequestRef.current;
      const commit = (entries: ManagedEntity[]) => {
        if (!signal.aborted && request === listRequestRef.current)
          setEntities(entries);
      };
      setLoading(true);
      listLoading.current = true;

      try {
        if (tagIds !== undefined) {
          const entries = await platform.listTagEntities(
            signal,
            tagIds,
            filter ?? "unread"
          );
          commit(entries);
        } else if (isFeed(selectedList)) {
          const entries = await listEntries(selectedList, filter);
          if (entries) commit(entries);
        } else if (isTopic(selectedList)) {
          const entities = await listTopicEntities(
            selectedList.id,
            filter ?? "unread",
            topicSort
          );
          if (entities) commit(entities);
        } else if (isSite(selectedList)) {
          const entries = await listSiteEntries(selectedList.id, filter);
          if (entries) commit(entries);
        } else {
          const entries = await listAllEntities(filter);
          if (entries) commit(entries);
        }
      } catch (error) {
        if (!signal.aborted && request === listRequestRef.current)
          pushError(error);
      } finally {
        if (!signal.aborted && request === listRequestRef.current) {
          listLoading.current = false;
          setLoading(false);
        }
      }
    },
    [
      tagIds,
      platform,
      abortControllerRef,
      pushError,
      articleScopeId,
      requestArticleScope,
      filter,
      listEntries,
      listTopicEntities,
      listSiteEntries,
      listAllEntities,
      topicSort,
    ]
  );

  const handleRefresh = useCallback(async () => {
    if (tagIds !== undefined) {
      void requestEntities(selectedScope);
      return;
    }
    if (readOnly || refreshSpin || loadingFeeds) return;
    setRefreshSpin(true);
    try {
      await refreshAllFeeds();
    } finally {
      setRefreshSpin(false);
    }
  }, [
    tagIds,
    requestEntities,
    selectedScope,
    refreshAllFeeds,
    refreshSpin,
    loadingFeeds,
    readOnly,
  ]);

  const handleFilter = useCallback(
    (_event: MouseEvent<HTMLElement>, value: string) => {
      if (isFilter(value)) onFilterChange(value);
    },
    [onFilterChange]
  );

  const handleMarkEntriesRead = useCallback(async () => {
    if (readOnly) return;
    await Promise.all(
      displayEntities
        .filter(isEntry)
        .map((entry) => entry.id)
        .map(markEntryRead)
    );
  }, [displayEntities, markEntryRead, readOnly]);

  const selectedScopeRef = useRef(selectedScope);
  selectedScopeRef.current = selectedScope;

  const mergeAddedTopicEntity = useCallback(
    async (changedTopicId: string, entityId: string) => {
      if (changedTopicId !== topicId) return;
      const authoritative = await listTopicEntities(
        changedTopicId,
        filter ?? "unread",
        topicSort
      );
      if (
        !authoritative ||
        !isTopic(selectedScopeRef.current) ||
        selectedScopeRef.current.id !== changedTopicId
      )
        return;
      setEntities((current) =>
        mergeTopicAddition(current, authoritative, entityId)
      );
    },
    [topicId, filter, topicSort, listTopicEntities]
  );

  const tagTextFilter = tagIds !== undefined ? filterText : undefined;
  const selectionRevision = useRef(state.tagSelectionRevision);
  useEffect(() => {
    abort();
    if (selectionRevision.current !== state.tagSelectionRevision) {
      const replaced = state.tagSelectionRevision !== undefined;
      selectionRevision.current = state.tagSelectionRevision;
      if (replaced) {
        ++listRequestRef.current;
        listLoading.current = false;
        setLoading(false);
        return;
      }
    }
    (async () => {
      requestEntities(selectedScopeRef.current);
    })();
    // Depend on IDs, not objects, so feed/topic identity changes from
    // requestFeeds/requestTopics don't re-trigger a full list reload.
  }, [
    tagSelectionKey,
    state.tagSelectionRevision,
    tagTextFilter,
    articleScopeId,
    feedId,
    topicId,
    siteHostname,
    shortcutId,
    requestEntities,
    abort,
  ]);

  useEffect(() => {
    if (lastLocationKey.current !== location.key) {
      lastLocationKey.current = location.key;
      if (navigationType === "POP")
        pendingHistorySelection.current = selectedEntityId;
      // Reader URL/presentation replacements may arrive while the list loads.
      else if (
        navigationType === "PUSH" ||
        pendingHistorySelection.current !== selectedEntityId
      )
        pendingHistorySelection.current = undefined;
    }
    const id = pendingHistorySelection.current;
    if (!id || listLoading.current) return;
    pendingHistorySelection.current = undefined;
    const index = displayEntities.findIndex((entity) => entity.id === id);
    if (index !== -1)
      virtuosoRef.current?.scrollIntoView({ index, behavior: "auto" });
  }, [
    location.key,
    navigationType,
    selectedEntityId,
    displayEntities,
    loading,
  ]);

  useEffect(() => {
    return onEmbeddingsChanged(() => {
      if (siteHostname) void requestEntities(selectedScopeRef.current);
    });
  }, [onEmbeddingsChanged, requestEntities, siteHostname]);

  useEffect(() => {
    return onUnreadChanged((entry) => {
      if (entry.id === articleScopeRef.current) {
        articleStateRef.current = {
          version: articleStateRef.current.version + 1,
          entry,
        };
      }
      setEntities((prev) => prev.map((p) => (p.id === entry.id ? entry : p)));
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
      setEntities((prev) => prev.map((p) => (p.id === entry.id ? entry : p)));
    });
  }, [onBookmarkChanged]);

  useEffect(() => {
    return onAnnotationChanged((entity) => {
      if (
        articleScopeId &&
        "reference" in entity &&
        entity.reference === articleScopeId
      ) {
        void requestArticleScope();
        return;
      }
      const active = allEntitiesMergeRef.current;
      setEntities((current) =>
        mergeAllEntitiesAnnotationChange(
          current,
          entity,
          active.shortcutId === ALL_ENTITIES.id,
          active.filter,
          active.filterText
        )
      );
      if (topicId) void mergeAddedTopicEntity(topicId, entity.id);
    });
  }, [
    onAnnotationChanged,
    topicId,
    mergeAddedTopicEntity,
    articleScopeId,
    requestArticleScope,
  ]);

  useEffect(() => {
    return onAnnotationReplaced(({ oldId, entity }) => {
      if (isPartialAnnotation(entity))
        replaceAnnotationSelection(oldId, entity);
      if (
        articleScopeId &&
        "reference" in entity &&
        entity.reference === articleScopeId
      ) {
        void requestArticleScope();
        return;
      }
      setEntities((prev) =>
        prev.map((value) => (value.id === oldId ? entity : value))
      );
    });
  }, [
    onAnnotationReplaced,
    articleScopeId,
    requestArticleScope,
    replaceAnnotationSelection,
  ]);

  useEffect(() => {
    return onAnnotationRemoved((id) => {
      replaceAnnotationSelection(id);
      if (articleScopeId) {
        void requestArticleScope();
        return;
      }
      setEntities((prev) => prev.filter((p) => p.id !== id));
    });
  }, [
    onAnnotationRemoved,
    articleScopeId,
    requestArticleScope,
    replaceAnnotationSelection,
  ]);

  useEffect(() => {
    return onFeedsRefreshed(() => {
      if (tagIds === undefined)
        return requestEntities(selectedScopeRef.current);
    });
  }, [onFeedsRefreshed, requestEntities, tagIds]);

  useEffect(
    () =>
      onTopicEntityAdded(async ({ topicId: changedTopicId, entityId }) => {
        await mergeAddedTopicEntity(changedTopicId, entityId);
      }),
    [mergeAddedTopicEntity]
  );

  function shortcutOptions(
    nodes: ShortcutNode[],
    depth = 0
  ): React.ReactNode[] {
    return nodes.flatMap((node): React.ReactNode[] => {
      const key = pinKey(node.reference);
      if (!node.children) {
        return [
          <MenuItem
            key={`pin:${key}`}
            value={`pin:${key}`}
            disabled={!node.target}
            sx={{ pl: 2 + depth * 2 }}
          >
            {node.title}
          </MenuItem>,
        ];
      }
      const children = node.children.length
        ? shortcutOptions(node.children, depth + 1)
        : [
            <MenuItem key={`empty:${key}`} disabled sx={{ pl: 4 + depth * 2 }}>
              Empty
            </MenuItem>,
          ];
      return [
        <ListSubheader key={key} sx={{ pl: 2 + depth * 2 }}>
          {node.title}
        </ListSubheader>,
        ...children,
      ];
    });
  }

  return (
    <div className={cx(className, css.EntityList)}>
      <div className={css.Header}>
        <div className={cx(css.HeadingWrapper)}>
          {!isSelectableHeader && (
            <>
              <Typography
                data-testid="all-items"
                className={css.Heading}
                component="div"
                color="text.secondary"
                variant="overline"
                sx={{ userSelect: "none" }}
              >
                {selectedScope?.title ?? "All items"}
              </Typography>

              {handleEdit && (
                <Tooltip title="Edit">
                  <IconButton
                    data-testid="edit-collection"
                    size="small"
                    onClick={handleEdit}
                  >
                    <Settings sx={{ fontSize: 16 }} />
                  </IconButton>
                </Tooltip>
              )}
            </>
          )}

          {isSelectableHeader && (
            <div className={css.HeadingControl}>
              <Tooltip
                title={
                  readOnly && tagIds === undefined
                    ? "This library is read-only"
                    : "Refresh"
                }
              >
                <span>
                  <IconButton
                    className={cx(
                      css.RefreshButton,
                      refreshSpin && css.RefreshLoading
                    )}
                    size="small"
                    data-testid="refresh-collection"
                    aria-label="Refresh collection"
                    disabled={
                      tagIds === undefined &&
                      (readOnly || refreshSpin || loadingFeeds)
                    }
                    onClick={handleRefresh}
                  >
                    <Refresh fontSize="small" />
                  </IconButton>
                </span>
              </Tooltip>

              <Select
                className={css.CollectionSelect}
                size="small"
                MenuProps={{
                  onKeyDownCapture: (event) => {
                    if (
                      event.key !== "Tab" ||
                      !event.currentTarget.contains(event.target as Node)
                    )
                      return;
                    const buttons = [
                      ...event.currentTarget.querySelectorAll<HTMLButtonElement>(
                        "[data-tag-action]"
                      ),
                    ];
                    const target = event.target as HTMLElement;
                    const current = buttons.indexOf(
                      target as HTMLButtonElement
                    );
                    const rowButton = target
                      .closest(".TagActionRow")
                      ?.querySelector<HTMLButtonElement>("[data-tag-action]");
                    const next =
                      current >= 0
                        ? buttons[current + (event.shiftKey ? -1 : 1)]
                        : event.shiftKey
                          ? undefined
                          : rowButton;
                    if (next) {
                      event.preventDefault();
                      event.stopPropagation();
                      next.focus();
                    }
                  },
                }}
                multiple
                open={headerOpen}
                onOpen={() => setHeaderOpen(true)}
                onClose={() => setHeaderOpen(false)}
                value={headerValues}
                renderValue={() => selectedScope.title}
                SelectDisplayProps={{
                  role: "combobox",
                  ...{ "data-testid": "collection-selector" },
                }}
                inputProps={{
                  "aria-label": "Browse collections and tags",
                }}
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
                {shortcutOptions(shortcutHierarchy)}
                {articleScopeId &&
                  !articleOptions.some(
                    (entry) => entry.id === articleScopeId
                  ) && (
                    <MenuItem value={`article:${articleScopeId}`} disabled>
                      Loading article…
                    </MenuItem>
                  )}
                {articleOptions
                  .filter(
                    (entry) =>
                      !shortcutTargets.some(
                        (target) => isEntry(target) && target.id === entry.id
                      )
                  )
                  .map((entry) => (
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
                <ListSubheader>Tags</ListSubheader>
                <MenuItem value="tags:all">All items (Tags)</MenuItem>
                {tagOptions.map((tag) => (
                  <MenuItem
                    className="TagActionRow"
                    key={`tag:${tag.id}`}
                    value={`tag:${tag.id}`}
                  >
                    <Checkbox
                      checked={tagIds?.includes(tag.id) ?? false}
                      tabIndex={-1}
                      slotProps={{ input: { "aria-hidden": true } }}
                    />
                    <Typography
                      component="span"
                      variant="body2"
                      sx={{ flexGrow: 1 }}
                    >
                      {tag.name}
                      {deleting.includes(tag.id)
                        ? " — Deleting…"
                        : merging.some((merge) => merge.survivorId === tag.id)
                          ? " — Merging…"
                          : renaming.includes(tag.id)
                            ? " — Renaming…"
                            : ""}
                    </Typography>
                    {tags.some((entity) => entity.id === tag.id) && (
                      <TagActions tag={tag} />
                    )}
                  </MenuItem>
                ))}
                {siteOptions.length > 0 && <ListSubheader>Sites</ListSubheader>}
                {siteOptions.map((site) => (
                  <MenuItem key={site.id} value={site.id}>
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
            data-testid="filter"
            aria-label="Filter"
            className={css.FilterButtonGroup}
            exclusive
            value={filter}
            onChange={handleFilter}
          >
            {!hasSelectedTags && (
              <ToggleButton
                data-testid="filter-unread"
                size="small"
                value="unread"
                aria-label="Unread"
              >
                Unread
              </ToggleButton>
            )}
            <ToggleButton
              data-testid="filter-saved"
              size="small"
              value="saved"
              aria-label="Saved"
            >
              Saved
            </ToggleButton>
            <ToggleButton
              data-testid="filter-all"
              size="small"
              value="all"
              aria-label="All"
            >
              All
            </ToggleButton>
          </ToggleButtonGroup>
        )}

        {isTopic(selectedScope) && (
          <Select
            className={css.SortSelect}
            size="small"
            value={topicSort}
            variant="standard"
            onChange={(e) => setTopicSort(e.target.value as TopicSort)}
          >
            <MenuItem value="relevance">Sort by most relevant</MenuItem>
            <MenuItem value="timestamp">Sort by most recent</MenuItem>
          </Select>
        )}

        <TextField
          data-testid="filter-items"
          className={css.FilterInput}
          size="small"
          fullWidth
          placeholder="Filter items…"
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
          sx={{
            "& .MuiInputBase-input": {
              paddingLeft: "0px !important",
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

      {!loading && entities.length === 0 && (
        <div className={cx(css.List, css.Placeholder)}>
          <Typography component="div" variant="body2" color="text.disabled">
            {getEmptyMessage(articleScopeId ? "all" : (filter ?? "unread"), "")}
          </Typography>
        </div>
      )}

      {entities.length > 0 && displayEntities.length === 0 && (
        <div className={cx(css.List, css.Placeholder)}>
          <Typography component="div" variant="body2" color="text.disabled">
            {getEmptyMessage(
              articleScopeId ? "all" : (filter ?? "unread"),
              filterText
            )}
          </Typography>
        </div>
      )}

      {displayEntities.length > 0 && (
        <Virtuoso
          ref={virtuosoRef}
          className={css.List}
          data={displayEntities}
          itemContent={(_index, entity) => (
            <div key={entity.id} className={css.ListEntity}>
              <ListItemButton
                data-bookmark-color={
                  isEntry(entity) ? (entity.color ?? "") : undefined
                }
                ref={(node: HTMLDivElement | null) => {
                  if (node) itemRefs.current.set(entity.id, node);
                  else itemRefs.current.delete(entity.id);
                }}
                className={cx(
                  css.ListItemButton,
                  isEntry(entity) && css.ArticleItem,
                  isEntry(entity) && entity.color && css.Bookmarked,
                  isEntry(entity) && entity.color && colorCss[entity.color]
                )}
                selected={selectedEntityId === entity.id}
                tabIndex={
                  selectedEntityId === entity.id ||
                  (!displayEntities.some(
                    (value) => value.id === selectedEntityId
                  ) &&
                    displayEntities[0].id === entity.id)
                    ? 0
                    : -1
                }
                onKeyDown={handleEntityKeyDown}
                onClick={() => selectEntity(entity)}
              >
                <ListItemText
                  primary={
                    <>
                      <Quotes
                        quoted={isPartialAnnotation(entity)}
                        color={
                          isPartialAnnotation(entity) ? entity.color : undefined
                        }
                      >
                        {entity.title}
                      </Quotes>
                      {isEntry(entity) && entity.color && (
                        <span className={css.ScreenReaderOnly}>
                          {` Bookmarked, ${entity.color[0].toUpperCase()}${entity.color.slice(1)}`}
                        </span>
                      )}
                    </>
                  }
                  secondary={
                    <EntityStatus entity={entity} showBookmark={false} />
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

      {loading && entities.length === 0 && (
        <div className={cx(css.List, css.Placeholder)}>
          <Typography component="div" variant="body2" color="text.disabled">
            Loading...
          </Typography>
        </div>
      )}

      {!articleScopeId && (
        <div className={css.Toolbar}>
          <Tooltip
            title={readOnly ? "This library is read-only" : "Mark all read"}
          >
            <span>
              <Button
                data-testid="mark-all-read"
                disabled={readOnly}
                onClick={handleMarkEntriesRead}
              >
                Mark all read
              </Button>
            </span>
          </Tooltip>
        </div>
      )}
    </div>
  );
}
