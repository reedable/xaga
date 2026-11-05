import { useEffect, useMemo, useRef, useState } from "react";
import { usePlatform } from "../platform/Platform";
import { useShortcutTargets } from "../hooks/useShortcutTargets";
import { useErrorSink } from "../hooks/useErrorSink";
import useRouter from "../hooks/useRouter";
import { ALL_ENTITIES } from "../Shortcuts";
import { pinKey, type PinTarget } from "../pins";
import { isEntry, isFeed, isSite } from "../types";
import sortSidebarCollections from "../utils/sortSidebarCollections";
import MissingShortcutDialog from "./sidebar/MissingShortcutDialog";

/** Lives outside the collapsible sidebar so native navigation is always available. */
export default function ShortcutsMenu() {
  const platform = usePlatform();
  const { pushError } = useErrorSink();
  const { targets, missing, pending } = useShortcutTargets();
  const { selectShortcutTarget } = useRouter();
  const [unavailable, setUnavailable] = useState<PinTarget | null>(null);
  const sorted = useMemo(
    () => sortSidebarCollections(targets, "alphabetical"),
    [targets]
  );
  const current = useRef({
    sorted,
    missing,
    pending,
    selectShortcutTarget,
  });
  useEffect(() => {
    current.current = { sorted, missing, pending, selectShortcutTarget };
  }, [sorted, missing, pending, selectShortcutTarget]);
  const keys = useRef(new Map<string, string>());
  const queue = useRef(Promise.resolve());

  useEffect(() => {
    let active = true;
    let stop: (() => void) | undefined;
    void platform
      .onShortcutMenuSelected((key) => {
        if (!active) return;
        const state = current.current;
        if (key === "all") {
          state.selectShortcutTarget(ALL_ENTITIES);
          return;
        }
        const identity = keys.current.get(key);
        if (!identity) return;
        const target = state.sorted.find((target) => {
          const type = isEntry(target)
            ? "entry"
            : isFeed(target)
              ? "feed"
              : isSite(target)
                ? "site"
                : "topic";
          return `${type}:${target.id}` === identity;
        });
        if (target) {
          state.selectShortcutTarget(target);
        } else {
          const target = state.missing.find(
            (target) => pinKey(target) === identity
          );
          if (target && !state.pending(target)) setUnavailable(target);
        }
      })
      .then((unlisten) => {
        if (active) stop = unlisten;
        else unlisten();
      })
      .catch(pushError);
    return () => {
      active = false;
      stop?.();
    };
  }, [platform, pushError]);

  useEffect(() => {
    // A new projection gets fresh keys: delayed clicks cannot select another item
    // after a reorder, removal, or library transition.
    const generation = crypto.randomUUID();
    const projected = [
      ...sorted.map((target) => ({
        identity: `${isEntry(target) ? "entry" : isFeed(target) ? "feed" : isSite(target) ? "site" : "topic"}:${target.id}`,
        title: target.title,
        enabled: true,
      })),
      ...missing.map((target) => ({
        identity: pinKey(target),
        title: pending(target) ? "Loading shortcut…" : "Shortcut unavailable",
        enabled: !pending(target),
      })),
    ];
    const items = projected.map((item, index) => ({
      key: `${generation}:${index}`,
      title: item.title,
      enabled: item.enabled,
    }));
    keys.current = new Map(
      items.map((item, index) => [item.key, projected[index].identity])
    );
    let active = true;
    // Serialize IPC updates and skip projections superseded before they start.
    queue.current = queue.current
      .then(async () => {
        if (active) await platform.setShortcutsMenu(items);
      })
      .catch(pushError);
    return () => {
      active = false;
    };
  }, [platform, sorted, missing, pending, pushError]);

  return (
    unavailable && (
      <MissingShortcutDialog
        key={pinKey(unavailable)}
        target={unavailable}
        open
        onClose={() => setUnavailable(null)}
      />
    )
  );
}
