export type PinTarget =
  | { type: "entry" | "feed" | "topic"; id: string }
  | { type: "site"; hostname: string };
type PinGroup = {
  type: "group";
  id: string;
  title: string;
  children: PinNode[];
};
export type PinNode = PinTarget | PinGroup;
export type PinNodeRef = PinTarget | { type: "group"; id: string };
export type PinEdit =
  | { action: "createGroup"; id: string; title: string; parent: string | null }
  | { action: "renameGroup"; id: string; title: string }
  | { action: "removeGroup"; id: string }
  | { action: "move"; node: PinNodeRef; parent: string | null };
export type PinChange =
  | { target: PinTarget; pinned: boolean }
  | { edit: PinEdit };
export function pinTargets(nodes: PinNode[]): PinTarget[] {
  return nodes.flatMap((node) =>
    node.type === "group" ? pinTargets(node.children) : [node]
  );
}
export type ShortcutMenuItem = {
  key: string;
  title: string;
  enabled: boolean;
  children?: ShortcutMenuItem[];
};
export function pinKey(target: PinNodeRef): string {
  return `${target.type}:${target.type === "site" ? target.hostname : target.id}`;
}
