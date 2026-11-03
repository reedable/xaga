import { useEffect, useMemo, useState, type ReactNode } from "react";
import { usePlatform } from "../platform/Platform";
import { useErrorSink } from "../hooks/useErrorSink";
import { subscribe } from "../hooks/useEvents";
import useRouter from "../hooks/useRouter";
import type { Tag } from "../domain";
import { debounced } from "../utils/Decorator";
import { TagsContext } from "./TagsContext";

export function TagsProvider({ children, enabled = true }: { children: ReactNode; enabled?: boolean }) {
  const platform = usePlatform();
  const { pushError } = useErrorSink();
  const { state, removeDeletedTags } = useRouter();
  const selection = JSON.stringify(state.tagIds ?? []);
  const [tags, setTags] = useState<Tag[]>([]);
  const [renaming, setRenaming] = useState<string[]>([]);
  const [deleting, setDeleting] = useState<string[]>([]);
  const [deletedIds, setDeletedIds] = useState<string[]>([]);
  const [hasLoaded, setLoaded] = useState(false);
  const [count, setCount] = useState<{ selection: string; value: number } | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    let revision = 0;
    const refresh = async () => {
      if (controller.signal.aborted) return;
      const request = ++revision;
      try {
        const result = await platform.readTagStatus(controller.signal);
        if (!controller.signal.aborted && request === revision) {
          setTags(result.tags);
          setRenaming(result.renaming);
          setDeleting(result.deleting);
          setDeletedIds((ids) => [...new Set([...ids, ...result.deleting])]);
          setLoaded(true);
        }
      } catch (error) { if (!controller.signal.aborted) pushError(error); }
    };
    void refresh();
    const refreshAfterChange = debounced(refresh, { wait: 400 });
    const stops = [
      subscribe(() => platform.onTagsChanged((snapshot) => {
        if (controller.signal.aborted) return;
        ++revision;
        setTags(snapshot.tags);
        setRenaming(snapshot.renaming);
        setDeleting(snapshot.deleting);
        setDeletedIds((ids) => [...new Set([...ids, ...snapshot.deleting])]);
        setLoaded(true);
      })),
      subscribe(() => platform.onTagCreated(() => void refresh())),
      subscribe(() => platform.onEmbeddingsChanged(refreshAfterChange)),
    ];
    return () => { controller.abort(); stops.forEach((stop) => stop()); };
  }, [platform, pushError]);

  useEffect(() => {
    if (!enabled) return;
    const controller = new AbortController();
    let revision = 0;
    const refresh = async () => {
      if (controller.signal.aborted) return;
      const request = ++revision;
      try {
        const value = await platform.countTagUnread(controller.signal, JSON.parse(selection));
        if (!controller.signal.aborted && request === revision) setCount({ selection, value });
      } catch (error) {
        if (!controller.signal.aborted && request === revision) { setCount(null); pushError(error); }
      }
    };
    void refresh();
    const refreshAfterChange = debounced(refresh, { wait: 400 });
    const stops = [
      subscribe(() => platform.onUnreadChanged(refreshAfterChange)),
      subscribe(() => platform.onEmbeddingsChanged(refreshAfterChange)),
    ];
    return () => { controller.abort(); stops.forEach((stop) => stop()); };
  }, [enabled, platform, pushError, selection]);

  useEffect(() => {
    if (deletedIds.some((id) => state.tagIds?.includes(id))) void removeDeletedTags(deletedIds);
  }, [deletedIds, state.tagIds, removeDeletedTags]);

  const unreadCount = enabled && count?.selection === selection ? count.value : null;
  const value = useMemo(() => ({ tags, hasLoaded, unreadCount, renaming, deleting }), [tags, hasLoaded, unreadCount, renaming, deleting]);
  return <TagsContext.Provider value={value}>{children}</TagsContext.Provider>;
}
