import { useCallback, useEffect, useMemo } from "react";
import { useLocation, useNavigate, useSearchParams } from "react-router-dom";
import { ALL_ITEMS } from "../Shortcuts";
import {
  isAnnotation,
  isCollection,
  isEntry,
  isFeed,
  isFilter,
  isPartialAnnotation,
  isTopic,
  type Annotation,
  type Countable,
  type Entry,
  type Feed,
  type Filter,
  type Item,
  type NavigationState,
  type Site,
  type Topic,
} from "../types";
import { decorate, logged } from "../utils/Decorator";
import Logger from "../utils/Logger";
import validateSearchInput from "../utils/validateSearchInput";

const logger = Logger.getLogger("useRouter");

let maxHistoryIdx = 0;

// True when two routes would show the same content, so navigating from one to
// the other is a no-op that should not grow the history. mimeType is excluded:
// it is derived by the content loader and restored via replaceMimeType, so two
// routes differing only in mimeType point at the same content.
function isSameNavigation(a: NavigationState, b: NavigationState) {
  return (
    a.topicId === b.topicId &&
    a.feedId === b.feedId &&
    a.siteHostname === b.siteHostname &&
    a.shortcutId === b.shortcutId &&
    a.entryId === b.entryId &&
    a.annotationId === b.annotationId &&
    a.url === b.url &&
    a.filter === b.filter
  );
}

export default function useRouter() {
  const navigate = useNavigate();
  const { state: rawState } = useLocation();
  const [searchParams] = useSearchParams();
  const rawSearch = searchParams.get("search");
  const search = useMemo(() => validateSearchInput(rawSearch), [rawSearch]);

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
    (topic: Topic, filter?: Filter) => {
      // Carry the active filter forward so it stays sticky across
      // collection switches.
      const next: NavigationState = {
        topicId: topic.id,
        filter: filter ?? state.filter,
      };
      if (isSameNavigation(state, next)) return;
      return logged_navigate(`/?${searchParams.toString()}`, { state: next });
    },
    [logged_navigate, searchParams, state]
  );

  const selectFeed = useCallback(
    (feed: Feed, filter?: Filter) => {
      const next: NavigationState = {
        feedId: feed.id,
        filter: filter ?? state.filter,
      };
      if (isSameNavigation(state, next)) return;
      return logged_navigate(`/?${searchParams.toString()}`, { state: next });
    },
    [logged_navigate, searchParams, state]
  );

  const selectSite = useCallback(
    (site: Site, filter?: Filter) => {
      const next: NavigationState = {
        siteHostname: site.hostname,
        filter: filter ?? state.filter,
      };
      if (isSameNavigation(state, next)) return;
      return logged_navigate(`/?${searchParams.toString()}`, { state: next });
    },
    [logged_navigate, searchParams, state]
  );

  const selectCollection = useCallback(
    (collection: Countable, filter?: Filter) => {
      if (isFeed(collection)) return selectFeed(collection, filter);
      if (isTopic(collection)) return selectTopic(collection, filter);

      const next: NavigationState = {
        shortcutId: collection.id,
        filter: filter ?? state.filter,
      };
      if (isSameNavigation(state, next)) return;
      return logged_navigate(`/?${searchParams.toString()}`, { state: next });
    },
    [logged_navigate, searchParams, selectFeed, selectTopic, state]
  );

  const selectEntry = useCallback(
    (entry: Entry) => {
      const next: NavigationState = {
        ...state,
        entryId: entry.id,
        url: entry.url,
        annotationId: undefined,
        mimeType: undefined, // article_loader to determine mimeType
      };
      if (isSameNavigation(state, next)) return;
      return logged_navigate(`/?${searchParams.toString()}`, { state: next });
    },
    [logged_navigate, searchParams, state]
  );

  const selectAnnotation = useCallback(
    (annotation: Pick<Annotation, "id" | "reference">) => {
      const next: NavigationState = {
        ...state,
        annotationId: annotation.id,
        entryId: annotation.reference,
        url: undefined,
        mimeType: undefined, // article_loader to determine mimeType
      };
      if (isSameNavigation(state, next)) return;
      return logged_navigate(`/?${searchParams.toString()}`, { state: next });
    },
    [logged_navigate, searchParams, state]
  );

  const selectUrl = useCallback(
    (url: string) => {
      const next: NavigationState = {
        ...state,
        url,
        entryId: undefined,
        annotationId: undefined,
        mimeType: undefined, // article_loader to determine mimeType
      };
      if (isSameNavigation(state, next)) return;
      logged_navigate(`/?${searchParams.toString()}`, { state: next });
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
        state: { ...state, entryId: entry.id, mimeType: undefined },
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

  const replaceMimeType = useCallback(
    (mimeType: string) => {
      if (mimeType === state.mimeType && !state.entryId) return;
      logged_navigate(`/?${searchParams.toString()}`, {
        replace: true,
        state: { ...state, mimeType, entryId: undefined },
      });
    },
    [logged_navigate, searchParams, state]
  );

  const selectItem = useCallback(
    (item: Item) => {
      if (isCollection(item)) return selectCollection(item);
      if (isEntry(item)) return selectEntry(item);
      if (isAnnotation(item) || isPartialAnnotation(item))
        return selectAnnotation(item);
      logger.error("Unknown item type", item);
    },
    [selectAnnotation, selectCollection, selectEntry]
  );

  const applyFilter = useCallback(
    (filter: string) => {
      const next = isFilter(filter) ? { ...state, filter } : state;
      if (isSameNavigation(state, next)) return;
      logged_navigate(`/?${searchParams.toString()}`, { state: next });
    },
    [logged_navigate, searchParams, state]
  );

  const invokeSearch = useCallback(
    (searchQuery: string) => {
      const params = new URLSearchParams(searchParams);
      params.set("search", searchQuery);
      logged_navigate(`/?${params.toString()}`, { replace: true, state });
    },
    [logged_navigate, searchParams, state]
  );

  const clearSearch = useCallback(() => {
    const params = new URLSearchParams(searchParams);
    params.delete("search");
    logged_navigate(`/?${params.toString()}`, { replace: true, state });
  }, [logged_navigate, searchParams, state]);

  const goBack = useCallback(() => navigate(-1), [navigate]);
  const goForward = useCallback(() => navigate(1), [navigate]);

  useEffect(() => {
    maxHistoryIdx = Math.max(maxHistoryIdx, currentIdx);
  }, [currentIdx]);

  return {
    filter: state.filter,
    canGoBack,
    canGoForward,
    search,
    state,

    clearSearch,
    goBack,
    goForward,
    invokeSearch,
    replaceEntry,
    replaceMimeType,
    replaceUrl,
    selectAnnotation,
    selectItem,
    selectCollection,
    selectFeed,
    selectSite,
    selectFilter: applyFilter,
    selectTopic,
    selectUrl,
  };
}
