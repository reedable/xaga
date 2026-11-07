import { useEffect, useRef, useState } from "react";
import { usePlatform } from "../platform/Platform";
import { useShortcutTargets } from "../hooks/useShortcutTargets";
import { useErrorSink } from "../hooks/useErrorSink";
import useRouter from "../hooks/useRouter";
import { ALL_ENTITIES } from "../Shortcuts";
import { pinKey, type PinTarget, type ShortcutMenuItem } from "../pins";
import { shortcutReference, type ShortcutNode } from "../utils/projectPinTree";
import MissingShortcutDialog from "./sidebar/MissingShortcutDialog";

/** Lives outside the collapsible sidebar so native navigation is always available. */
export default function ShortcutsMenu() {
  const platform = usePlatform();
  const { pushError } = useErrorSink();
  const { targets, missing, pending, hierarchy } = useShortcutTargets();
  const { selectShortcutTarget } = useRouter();
  const [unavailable, setUnavailable] = useState<PinTarget | null>(null);
  const current = useRef({
    targets,
    missing,
    pending,
    selectShortcutTarget,
  });
  useEffect(() => {
    current.current = { targets, missing, pending, selectShortcutTarget };
  }, [targets, missing, pending, selectShortcutTarget]);
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
        const target = state.targets.find(
          (target) => pinKey(shortcutReference(target)) === identity
        );
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
    const nodes = hierarchy;
    const identities = new Map<string, string>();
    let index = 0;
    const project = (nodes: ShortcutNode[]): ShortcutMenuItem[] =>
      nodes.map((node) => {
        const key = `${generation}:${index++}`;
        if (node.children)
          return {
            key,
            title: node.title,
            enabled: true,
            children: project(node.children),
          };
        identities.set(key, pinKey(node.reference));
        return { key, title: node.title, enabled: !node.pending };
      });
    const items = project(nodes);
    keys.current = identities;
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
  }, [platform, targets, missing, pending, hierarchy, pushError]);

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
