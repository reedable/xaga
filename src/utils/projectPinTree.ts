import { pinKey, type PinNode, type PinNodeRef, type PinTarget } from "../pins";
import type { Collection, Countable, Entry } from "../types";
import type { SidebarSortMode } from "../hooks/useSidebarSort";
import sortSidebarCollections from "./sortSidebarCollections";

export type ShortcutTarget = Collection | (Entry & Countable);
export type ShortcutNode = {
  reference: PinNodeRef;
  title: string;
  unreadCount: number;
  target?: ShortcutTarget;
  children?: ShortcutNode[];
  pending?: boolean;
};
export function shortcutReference(target: ShortcutTarget): PinTarget {
  return target.__kind === "site"
    ? { type: "site", hostname: target.id }
    : { type: target.__kind, id: target.id };
}

/** One projection supplies hierarchy, recursive counts and sorting to all views. */
export function projectPinTree(
  nodes: PinNode[],
  targets: ShortcutTarget[],
  mode: SidebarSortMode,
  pending: (target: PinTarget) => boolean
): ShortcutNode[] {
  const resolved = new Map(
    targets.map((target) => [pinKey(shortcutReference(target)), target])
  );
  function visit(nodes: PinNode[]): ShortcutNode[] {
    return sortSidebarCollections(
      nodes.map((node): ShortcutNode => {
        if (node.type === "group") {
          const children = visit(node.children);
          return {
            reference: { type: "group", id: node.id },
            title: node.title,
            unreadCount: children.reduce(
              (sum, child) => sum + child.unreadCount,
              0
            ),
            children,
          };
        }
        const target = resolved.get(pinKey(node));
        return {
          reference: node,
          target,
          title:
            target?.title ??
            (pending(node) ? "Loading shortcut…" : "Shortcut unavailable"),
          unreadCount:
            target && target.__kind !== "entry" ? target.unreadCount : 0,
          pending: !target && pending(node),
        };
      }),
      mode
    );
  }
  return visit(nodes);
}

export function filterPinTree(
  nodes: ShortcutNode[],
  filter: string
): ShortcutNode[] {
  const text = filter.trim().toLocaleLowerCase();
  if (!text) return nodes;
  return nodes.flatMap((node) => {
    if (
      node.title.toLocaleLowerCase().includes(text) ||
      (!node.target &&
        !node.children &&
        pinKey(node.reference).toLocaleLowerCase().includes(text))
    )
      return [node];
    const children = node.children && filterPinTree(node.children, text);
    return children?.length ? [{ ...node, children }] : [];
  });
}
