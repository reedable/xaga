import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
} from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { useReadingPane } from "../providers/ReadingPaneContext";
import { useNavigationState } from "../providers/NavigationContext";
import {
  isAnnotation,
  isCollection,
  isEntry,
  isPartialAnnotation,
  isShortcut,
  type Annotation,
  type Collection,
  type Entry,
  type Feed,
  type ManagedEntity,
  type NavigationState,
  type Site,
  type Shortcut,
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
    JSON.stringify(a.tagIds) === JSON.stringify(b.tagIds) &&
    a.articleScopeId === b.articleScopeId &&
    a.topicId === b.topicId &&
    a.feedId === b.feedId &&
    a.siteHostname === b.siteHostname &&
    a.shortcutId === b.shortcutId &&
    a.entryId === b.entryId &&
    a.annotationId === b.annotationId &&
    a.url === b.url
  );
}

export default function useRouter() {
  const navigate = useNavigate();
  const readingPane = useReadingPane();
  const activateContent = readingPane?.activate;
  const { state, selectCollectionState } = useNavigationState();
  const [searchParams] = useSearchParams();
  const rawSearch = searchParams.get("search");
  const search = useMemo(() => validateSearchInput(rawSearch), [rawSearch]);

  const stateRef = useRef(state);
  useLayoutEffect(() => {
    stateRef.current = state;
  }, [state]);

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

  const selectTags = useCallback(
    (ids: string[]) => {
      const next: NavigationState = {
        tagIds: [...new Set(ids)].sort(),
      };
      if (
        isSameNavigation(state, next) &&
        state.tagSelectionRevision === undefined
      )
        return;
      return logged_navigate(`/?${searchParams.toString()}`, { state: next });
    },
    [logged_navigate, searchParams, state]
  );

  // Committed tag identity changes preserve the current list and reader.
  const replaceTagSelection = useCallback(
    (ids: string[]) => {
      const tagIds = [...new Set(ids)].sort();
      if (
        !state.tagIds ||
        JSON.stringify([...state.tagIds].sort()) === JSON.stringify(tagIds)
      )
        return;
      return logged_navigate(`/?${searchParams.toString()}`, {
        replace: true,
        state: {
          ...state,
          tagIds,
          tagSelectionRevision: (state.tagSelectionRevision ?? 0) + 1,
        },
      });
    },
    [logged_navigate, searchParams, state]
  );

  const selectCollection = useCallback(
    (collection: Collection) => {
      const next: NavigationState = {
        ...(collection.__kind === "feed"
          ? { feedId: collection.id }
          : collection.__kind === "topic"
            ? { topicId: collection.id }
            : { siteHostname: collection.id }),
      };
      if (isSameNavigation(state, next)) return;
      return selectCollectionState(next);
    },
    [selectCollectionState, state]
  );

  const selectTopic = useCallback(
    (topic: Topic) => selectCollection(topic),
    [selectCollection]
  );

  const selectFeed = useCallback(
    (feed: Feed) => selectCollection(feed),
    [selectCollection]
  );

  const selectSite = useCallback(
    (site: Site) => selectCollection(site),
    [selectCollection]
  );

  const selectShortcut = useCallback(
    (shortcut: Shortcut) => {
      const next: NavigationState = {
        shortcutId: shortcut.id,
      };
      if (isSameNavigation(state, next)) return;
      return selectCollectionState(next);
    },
    [selectCollectionState, state]
  );

  const selectArticleScope = useCallback(
    (entry: Entry) => {
      activateContent?.();
      const next: NavigationState = {
        articleScopeId: entry.id,
        entryId: entry.id,
        url: entry.url,
      };
      if (isSameNavigation(state, next)) return;
      return logged_navigate(`/?${searchParams.toString()}`, { state: next });
    },
    [logged_navigate, searchParams, state, activateContent]
  );

  const selectShortcutTarget = useCallback(
    (target: Collection | Entry | Shortcut) =>
      isEntry(target)
        ? selectArticleScope(target)
        : isShortcut(target)
          ? selectShortcut(target)
          : selectCollection(target),
    [selectArticleScope, selectShortcut, selectCollection]
  );

  const selectEntry = useCallback(
    (entry: Entry) => {
      activateContent?.();
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
    [logged_navigate, searchParams, state, activateContent]
  );

  const selectAnnotation = useCallback(
    (annotation: Pick<Annotation, "id" | "reference"> & { url?: string }) => {
      activateContent?.();
      const sameArticle = state.entryId === annotation.reference;
      const next: NavigationState = {
        ...state,
        annotationId: annotation.id,
        entryId: annotation.reference,
        // Keep the resolved presentation when only the passage changes.
        // Clearing it makes the reader backfill the URL and render again.
        url: (sameArticle ? state.url : undefined) || annotation.url,
        mimeType: sameArticle ? state.mimeType : undefined,
      };
      if (isSameNavigation(state, next)) return;
      return logged_navigate(`/?${searchParams.toString()}`, { state: next });
    },
    [logged_navigate, searchParams, state, activateContent]
  );

  const replaceAnnotationSelection = useCallback(
    (oldId: string, replacement?: Pick<Annotation, "id" | "reference">) => {
      const current = stateRef.current;
      if (current.annotationId !== oldId) return;
      if (replacement && replacement.reference !== current.entryId) return;
      const next = { ...current, annotationId: replacement?.id };
      stateRef.current = next;
      return logged_navigate(`/?${searchParams.toString()}`, {
        replace: true,
        state: next,
      });
    },
    [logged_navigate, searchParams]
  );

  const selectUrl = useCallback(
    (url: string) => {
      activateContent?.();
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
    [logged_navigate, searchParams, state, activateContent]
  );

  // History-replaces the current route so it points at a saved Entry, without
  // pushing a new history entry. Used after a link followed within the reader
  // is loaded by URL and persisted: the URL is already current, but the route
  // lacks the entryId that annotations and related entities depend on.
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

  const selectEntity = useCallback(
    (entity: ManagedEntity | Collection) => {
      if (isCollection(entity)) return selectCollection(entity);
      if (isShortcut(entity)) return selectShortcut(entity);
      if (isEntry(entity)) return selectEntry(entity);
      if (isAnnotation(entity) || isPartialAnnotation(entity))
        return selectAnnotation(entity);
      logger.error("Unknown entity type", entity);
    },
    [selectAnnotation, selectCollection, selectEntry, selectShortcut]
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
    replaceAnnotationSelection,
    selectArticleScope,
    selectEntity,
    selectCollection,
    selectShortcut,
    selectShortcutTarget,
    selectFeed,
    selectSite,
    selectTopic,
    selectTags,
    replaceTagSelection,
    selectUrl,
  };
}
