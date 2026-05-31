import { useCallback } from "react";
import { useLocation, useNavigate, useSearchParams } from "react-router-dom";
import { ALL_ITEMS } from "../Shortcuts";
import {
  isAnnotation,
  isAnnotationSummary,
  isCollection,
  isEntry,
  isTopic,
  type Annotation,
  type Collection,
  type Entry,
  type Item,
  type NavigationState,
} from "../types";
import { decorate, logged } from "../utils/Decorator";
import Logger from "../utils/Logger";

const logger = Logger.getLogger("useRouter");

let maxHistoryIdx = 0;

export default function useRouter() {
  const navigate = useNavigate();
  const { state: rawState } = useLocation();
  const state: NavigationState = (rawState as NavigationState | null) ?? {
    shortcutId: ALL_ITEMS.id,
  };
  const [searchParams, setSearchParams] = useSearchParams();

  const currentIdx =
    (window.history.state as { idx?: number } | null)?.idx ?? 0;
  maxHistoryIdx = Math.max(maxHistoryIdx, currentIdx);
  const canGoBack = currentIdx > 0;
  const canGoForward = currentIdx < maxHistoryIdx;

  const logged_navigate = decorate(navigate).with(logged, {
    logger,
    name: "navigate",
  });

  const showItem = useCallback(
    (item: Item) => {
      const params = new URLSearchParams(searchParams);
      let entryId: Entry["id"];
      let annotationId: Annotation["id"] | undefined;
      let url: Entry["url"] | undefined;

      if (isAnnotation(item)) {
        annotationId = item.id;
        entryId = item.reference;
        if (isAnnotationSummary(item)) url = item.url;
      } else {
        entryId = item.id;
        if (isEntry(item)) url = item.url;
      }

      logged_navigate(`/?${params.toString()}`, {
        state: { ...state, entryId, annotationId, url },
      });
    },
    [logged_navigate, searchParams, state]
  );

  const showCollection = useCallback(
    (collection: Collection) => {
      if (isTopic(collection)) {
        const params = new URLSearchParams(searchParams);
        logged_navigate(`/?${params.toString()}`, {
          state: { topicId: collection.id },
        });
      } else {
        logger.warn("Unhandled collection type", collection);
      }
    },
    [logged_navigate, searchParams]
  );

  const show = useCallback(
    (item: Item) => {
      if (isCollection(item)) {
        showCollection(item);
      } else {
        showItem(item);
      }
    },
    [showCollection, showItem]
  );

  const openUrl = useCallback(
    (url: string) => {
      const params = new URLSearchParams(searchParams);
      logged_navigate(`/?${params.toString()}`, {
        state: { ...state, url },
      });
    },
    [logged_navigate, searchParams, state]
  );

  const search = useCallback(
    (searchString: string) => {
      const params = new URLSearchParams(searchParams);
      params.set("search", searchString);
      logged_navigate(`/?${params.toString()}`, { state });
    },
    [logged_navigate, searchParams, state]
  );

  const clearSearch = useCallback(() => {
    const params = new URLSearchParams(searchParams);
    params.delete("search");
    logged_navigate(`/?${params.toString()}`, { state });
  }, [logged_navigate, searchParams, state]);

  const back = useCallback(() => navigate(-1), [navigate]);
  const forward = useCallback(() => navigate(1), [navigate]);

  return {
    navigate: logged_navigate,
    back,
    forward,
    canGoBack,
    canGoForward,
    openUrl,
    search,
    clearSearch,
    show,
    state,
    searchParams,
    setSearchParams,
  };
}
