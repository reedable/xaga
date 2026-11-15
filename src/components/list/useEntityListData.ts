import { useShortcutTargets } from "../../hooks/useShortcutTargets";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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
  isFeed,
  isPartialAnnotation,
  isSite,
  isTopic,
  type Entry,
  type Filter,
  type ManagedEntity,
  type Site,
  type TopicSort,
} from "../../types";
import { articleScopeEntities } from "../../utils/articleScopeEntities";
import {
  entityMatchesTextFilter,
  mergeAllEntitiesAnnotationChange,
} from "../../utils/mergeAllEntitiesAnnotationChange";
import { mergeTopicAddition } from "../../utils/mergeTopicAddition";
import sortByTitle from "../../utils/sortByTitle";

type DataOptions = {
  state: ReturnType<typeof useRouter>["state"];
  listFilter: Filter;
  readOnly: boolean;
  replaceAnnotationSelection: ReturnType<
    typeof useRouter
  >["replaceAnnotationSelection"];
};

export function useEntityListData({
  state,
  listFilter,
  readOnly,
  replaceAnnotationSelection,
}: DataOptions) {
  const listLoading = useRef(false);
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

  const displayEntities = useMemo(() => {
    return entities.filter((entity) =>
      entityMatchesTextFilter(entity, filterText)
    );
  }, [entities, filterText]);

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

  return {
    tagIds,
    hasSelectedTags,
    filter,
    tags,
    renaming,
    deleting,
    merging,
    tagOptions,
    pinnedEntries,
    shortcutHierarchy,
    shortcutTargets,
    articleScopeId,
    feedId,
    topicId,
    siteHostname,
    topics,
    feeds,
    loadingFeeds,
    missingPins,
    loading,
    refreshSpin,
    selectedScope,
    siteOptions,
    articleOptions,
    entities,
    filterText,
    setFilterText,
    topicSort,
    setTopicSort,
    displayEntities,
    listLoading,
    handleRefresh,
    handleMarkEntriesRead,
  };
}
