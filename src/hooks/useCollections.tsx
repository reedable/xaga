import { useCallback, useContext, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import CollectionsContext from "../providers/CollectionsContext";
import type { Annotation, Collection, Entry, Filter, SavedItem } from "../types";
import { useSidebar } from "./useSidebar";

export function useCollections(abortControllerRef: RefObject<AbortController>) {
  const ctx = useContext(CollectionsContext);
  if (!ctx) throw new Error("useCollections must be used inside CollectionsProvider");

  const { collections, pinnedCollectionIds, setCollections, setPinnedCollectionIds } = ctx;

  const platform = usePlatform();
  const { listCollections } = useSidebar(abortControllerRef);

  const requestCollections = useCallback(async () => {
    const results = await listCollections();
    if (results) setCollections(results);
  }, [listCollections, setCollections]);

  const createCollection = useCallback(
    async (title: string) => {
      const signal = abortControllerRef.current.signal;
      try {
        return await platform.createCollection(signal, title);
      } catch {
        // surfaced via dialog
      }
    },
    [abortControllerRef, platform]
  );

  const saveCollection = useCallback(
    async (collection: Collection) => {
      const signal = abortControllerRef.current.signal;
      try {
        return await platform.saveCollection(signal, collection.id, collection.title);
      } catch {
        // surfaced via dialog
      }
    },
    [abortControllerRef, platform]
  );

  const deleteCollection = useCallback(
    async (id: string) => {
      const signal = abortControllerRef.current.signal;
      try {
        await platform.deleteCollection(signal, id);
      } catch {
        // surfaced via dialog
      }
    },
    [abortControllerRef, platform]
  );

  const listEntryCollections = useCallback(
    async (entryId: string): Promise<string[] | undefined> => {
      const signal = abortControllerRef.current.signal;
      try {
        return await platform.listEntryCollections(signal, entryId);
      } catch {
        // surfaced by caller
      }
    },
    [abortControllerRef, platform]
  );

  const addCollectionItem = useCallback(
    async (collectionId: string, item: Entry | Annotation) => {
      const signal = abortControllerRef.current.signal;
      try {
        return await platform.addCollectionItem(signal, collectionId, item);
      } catch {
        // surfaced by caller
      }
    },
    [abortControllerRef, platform]
  );

  const removeCollectionItem = useCallback(
    async (collectionId: string, itemId: string) => {
      const signal = abortControllerRef.current.signal;
      try {
        return await platform.removeCollectionItem(signal, collectionId, itemId);
      } catch {
        // surfaced by caller
      }
    },
    [abortControllerRef, platform]
  );

  const listCollectionItems = useCallback(
    async (id: string, filter: Filter): Promise<SavedItem[] | undefined> => {
      const signal = abortControllerRef.current.signal;
      try {
        return await platform.listCollectionItems(signal, id, filter);
      } catch {
        // surfaced via ItemList
      }
    },
    [abortControllerRef, platform]
  );

  return {
    collections,
    pinnedCollectionIds,
    setPinnedCollectionIds,
    requestCollections,
    createCollection,
    saveCollection,
    deleteCollection,
    listCollectionItems,
    listEntryCollections,
    addCollectionItem,
    removeCollectionItem,
  };
}
