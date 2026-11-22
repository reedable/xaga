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
export type PinPlacement = { parent: string | null; moveExisting: boolean };
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

export function listShortcutDestinations(
  nodes: PinNode[],
  excludedGroup?: string
): { id: string; title: string }[] {
  const visit = (
    children: PinNode[],
    path: string[]
  ): { id: string; title: string }[] =>
    children.flatMap((node) => {
      if (node.type !== "group" || node.id === excludedGroup) return [];
      const segments = [...path, node.title];
      return [
        {
          id: node.id,
          title: segments
            .map((segment) =>
              /[/"\\]/.test(segment) ? JSON.stringify(segment) : segment
            )
            .join(" / "),
        },
        ...visit(node.children, segments),
      ];
    });
  const groups = visit(nodes, []);
  const counts = new Map<string, number>();
  for (const { title } of groups)
    counts.set(title, (counts.get(title) ?? 0) + 1);
  // Reserve unique path labels so the disambiguator cannot collide with a title.
  const reserved = new Set([
    "No shortcut",
    "Top level",
    "Top level of Shortcuts",
    "Folder unavailable",
  ]);
  const used = new Set([...counts.keys(), ...reserved]);
  return groups.map(({ id, title }) => {
    if (counts.get(title) === 1 && !reserved.has(title)) return { id, title };
    let label = `${title} (${id})`;
    while (used.has(label)) label = `${label} (${id})`;
    used.add(label);
    return { id, title: label };
  });
}
