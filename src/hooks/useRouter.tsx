import { useCallback } from "react";
import { useLocation, useNavigate, useSearchParams } from "react-router-dom";
import {
  isAnnotation,
  isCollection,
  type Annotation,
  type Collection,
  type Entry,
  type Item,
  type NavigationState,
} from "../types";
import { decorate, logged } from "../utils/Decorator";
import Logger from "../utils/Logger";

export function isNavigationState(value: unknown): value is NavigationState {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  return (
    (v.topicId === undefined || typeof v.topicId === "string") &&
    (v.feedId === undefined || typeof v.feedId === "string") &&
    (v.shortcutId === undefined || typeof v.shortcutId === "string") &&
    (v.entryId === undefined || typeof v.entryId === "string") &&
    (v.annotationId === undefined || typeof v.annotationId === "string")
  );
}

const logger = Logger.getLogger("useRouter");

export default function useRouter() {
  const navigate = useNavigate();
  const { state } = useLocation();
  const [searchParams, setSearchParams] = useSearchParams();

  const logged_navigate = decorate(navigate).with(logged, {
    logger,
    name: "navigate",
  });

  const showItem = useCallback(
    (item: Item) => {
      const params = new URLSearchParams(searchParams);
      let entryId: Entry["id"];
      let annotationId: Annotation["id"] | undefined;

      if (isAnnotation(item)) {
        annotationId = item.id;
        entryId = item.reference;
      } else {
        entryId = item.id;
      }

      logged_navigate(`/?${params.toString()}`, {
        state: { ...state, entryId, annotationId },
      });
    },
    [logged_navigate, searchParams, state]
  );

  const showCollection = useCallback((collection: Collection) => {
    logger.warn("Implement me!", collection);
  }, []);

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

  const search = useCallback(
    (searchString: string) => {
      const params = new URLSearchParams(searchParams);
      params.set("search", searchString);
      logged_navigate(`/?${params.toString()}`, { state });
    },
    [logged_navigate, searchParams, state]
  );

  return {
    navigate: logged_navigate,
    search,
    show,
    state,
    searchParams,
    setSearchParams,
  };
}
