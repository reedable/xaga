import { useCallback, useEffect, useMemo, useState } from "react";
import { useLocation, useNavigate, useSearchParams } from "react-router-dom";
import { ALL_ITEMS } from "../Shortcuts";
import {
  isAnnotation,
  isCollection,
  isEntry,
  isFeed,
  isFilter,
  isTopic,
  type Annotation,
  type Collection,
  type Entry,
  type Feed,
  type Item,
  type NavigationState,
  type Topic,
} from "../types";
import { decorate, logged } from "../utils/Decorator";
import Logger from "../utils/Logger";

const logger = Logger.getLogger("useRouter");

let maxHistoryIdx = 0;

export default function useRouter() {
  const navigate = useNavigate();
  const { state: rawState } = useLocation();
  const [searchParams] = useSearchParams();
  const [filter, setFilter] = useState(searchParams.get("filter") ?? undefined);
  const [search, setSearch] = useState(searchParams.get("search") ?? undefined);

  const state: NavigationState = useMemo(
    () => (rawState as NavigationState | null) ?? { shortcutId: ALL_ITEMS.id },
    [rawState]
  );

  const currentIdx =
    (window.history.state as { idx?: number } | null)?.idx ?? 0;

  const canGoBack = currentIdx > 0;
  const canGoForward = currentIdx < maxHistoryIdx;

  // Log activity while navigating
  const logged_navigate = useMemo(() => {
    return decorate(navigate).with(logged, {
      logger,
      name: "navigate",
    });
  }, [navigate]);

  const selectTopic = useCallback(
    (topic: Topic, filter?: string) => {
      if (isFilter(filter)) searchParams.set("filter", filter);
      return logged_navigate(`/?${searchParams.toString()}`, {
        state: { topicId: topic.id },
      });
    },
    [logged_navigate, searchParams]
  );

  const selectFeed = useCallback(
    (feed: Feed, filter?: string) => {
      if (isFilter(filter)) searchParams.set("filter", filter);
      return logged_navigate(`/?${searchParams.toString()}`, {
        state: { feedId: feed.id },
      });
    },
    [logged_navigate, searchParams]
  );

  const selectCollection = useCallback(
    (collection: Collection, filter?: string) => {
      if (isFeed(collection)) return selectFeed(collection, filter);
      if (isTopic(collection)) return selectTopic(collection, filter);

      if (isFilter(filter)) searchParams.set("filter", filter);
      return logged_navigate(`/?${searchParams.toString()}`, {
        state: { shortcutId: collection.id },
      });
    },
    [logged_navigate, searchParams, selectFeed, selectTopic]
  );

  const selectEntry = useCallback(
    (entry: Entry) => {
      return logged_navigate(`/?${searchParams.toString()}`, {
        state: {
          ...state,
          entryId: entry.id,
          url: entry.url,
          annotationId: undefined,
          mimeType: undefined, // article_loader to determine mimeType
        },
      });
    },
    [logged_navigate, searchParams, state]
  );

  const selectAnnotation = useCallback(
    (annotation: Annotation) => {
      return logged_navigate(`/?${searchParams.toString()}`, {
        state: {
          ...state,
          annotationId: annotation.id,
          entryId: annotation.reference,
          url: undefined,
          mimeType: undefined, // article_loader to determine mimeType
        },
      });
    },
    [logged_navigate, searchParams, state]
  );

  const selectUrl = useCallback(
    (url: string) => {
      logged_navigate(`/?${searchParams.toString()}`, {
        state: {
          ...state,
          url,
          entryId: undefined,
          annotationId: undefined,
          mimeType: undefined, // article_loader to determine mimeType
        },
      });
    },
    [logged_navigate, searchParams, state]
  );

  // History-replaces the current route so it points at a saved Entry, without
  // pushing a new history entry. Used after a link followed within the reader
  // is loaded by URL and persisted: the URL is already current, but the route
  // lacks the entryId that annotations and related items depend on.
  const replaceEntry = useCallback(
    (entry: Entry) =>
      logged_navigate(`/?${searchParams.toString()}`, {
        replace: true,
        state: { ...state, entryId: entry.id },
      }),
    [logged_navigate, searchParams, state]
  );

  // History-replaces the current route so it carries the given url, without
  // pushing a new history entry. Used after an annotation is selected: the
  // route has the entryId (annotation.reference) but no url, which the reader
  // needs to load the entry's content.
  const replaceUrl = useCallback(
    (url: string) =>
      logged_navigate(`/?${searchParams.toString()}`, {
        replace: true,
        state: { ...state, url },
      }),
    [logged_navigate, searchParams, state]
  );

  const selectItem = useCallback(
    (item: Item) => {
      if (isCollection(item)) return selectCollection(item);
      if (isEntry(item)) return selectEntry(item);
      if (isAnnotation(item)) return selectAnnotation(item);
      logger.error("Unknown item type", item);
    },
    [selectAnnotation, selectCollection, selectEntry]
  );

  const applyFilter = useCallback(
    (filter: string) => {
      if (isFilter(filter)) {
        searchParams.set("filter", filter);
      }

      logged_navigate(`/?${searchParams.toString()}`, {
        state,
      });
    },
    [logged_navigate, searchParams, state]
  );

  const invokeSearch = useCallback(
    (searchQuery: string) => {
      const params = new URLSearchParams(searchParams);
      params.set("search", searchQuery);
      logged_navigate(`/?${params.toString()}`, { state });
    },
    [logged_navigate, searchParams, state]
  );

  const clearSearch = useCallback(() => {
    const params = new URLSearchParams(searchParams);
    params.delete("search");
    logged_navigate(`/?${params.toString()}`, { state });
  }, [logged_navigate, searchParams, state]);

  const goBack = useCallback(() => navigate(-1), [navigate]);
  const goForward = useCallback(() => navigate(1), [navigate]);

  useEffect(() => {
    maxHistoryIdx = Math.max(maxHistoryIdx, currentIdx);
  }, [currentIdx]);

  useEffect(() => {
    (async () => {
      const filterParam = searchParams.get("filter");
      const searchParam = searchParams.get("search");
      if (isFilter(filterParam)) setFilter(filterParam);
      if (searchParam) setSearch(searchParam);
    })();
  }, [searchParams]);

  return {
    filter,
    canGoBack,
    canGoForward,
    search,
    state,

    clearSearch,
    goBack,
    goForward,
    invokeSearch,
    replaceEntry,
    replaceUrl,
    selectAnnotation,
    selectItem,
    selectCollection,
    selectFeed,
    selectFilter: applyFilter,
    selectTopic,
    selectUrl,
  };
}
