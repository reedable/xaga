import { expect, it } from "vitest";
import {
  filterPinTree,
  projectPinTree,
  type ShortcutTarget,
} from "./projectPinTree";
import type { PinNode } from "../pins";

const targets = [
  {
    __kind: "feed",
    id: "f",
    title: "Feed",
    createdAt: "2026-01-01",
    members: [],
    unreadCount: 4,
  },
  {
    __kind: "topic",
    id: "t",
    title: "Topic",
    createdAt: "2026-01-01",
    members: [],
    unreadCount: 4,
  },
  {
    __kind: "entry",
    id: "a",
    title: "Article",
    createdAt: "2026-01-01",
    url: "https://example.com/a",
    unreadCount: 100,
  },
] as ShortcutTarget[];
const pins: PinNode[] = [
  {
    type: "group",
    id: "g",
    title: "Research",
    children: [
      { type: "entry", id: "a" },
      {
        type: "group",
        id: "nested",
        title: "Background",
        children: [
          { type: "feed", id: "f" },
          { type: "topic", id: "t" },
        ],
      },
    ],
  },
  { type: "group", id: "empty", title: "Empty", children: [] },
  { type: "topic", id: "missing" },
];
it("sorts each container by recursive counts, counting collections separately and articles as zero", () => {
  const result = projectPinTree(pins, targets, "unread", () => false);
  expect(result.map((node) => [node.title, node.unreadCount])).toEqual([
    ["Research", 8],
    ["Empty", 0],
    ["Shortcut unavailable", 0],
  ]);
  expect(
    result[0].children?.map((node) => [node.title, node.unreadCount])
  ).toEqual([
    ["Background", 8],
    ["Article", 0],
  ]);
  expect(result[0].children?.[0].children?.map((node) => node.title)).toEqual([
    "Feed",
    "Topic",
  ]);
  expect(pins[0].type === "group" && pins[0].children[0].type).toBe("entry");
});
it("retains ancestor paths, reveals matching group subtrees, and leaves the full projection unchanged", () => {
  const tree = projectPinTree(pins, targets, "alphabetical", () => false);
  const filtered = filterPinTree(tree, "  feed ");
  expect(filtered.map((node) => node.title)).toEqual(["Research"]);
  expect(filtered[0].children?.map((node) => node.title)).toEqual([
    "Background",
  ]);
  expect(filtered[0].children?.[0].children?.map((node) => node.title)).toEqual(
    ["Feed"]
  );
  expect(filterPinTree(tree, "research")[0].children).toBe(
    tree.find((node) => node.title === "Research")?.children
  );
  expect(tree.find((node) => node.title === "Research")?.children).toHaveLength(
    2
  );
});
it("preserves unavailable references and loading state within their group", () => {
  const tree = projectPinTree(
    [
      {
        type: "group",
        id: "g",
        title: "Group",
        children: [{ type: "topic", id: "missing" }],
      },
    ],
    [],
    "unread",
    () => true
  );
  expect(tree[0].children?.[0]).toMatchObject({
    pending: true,
    title: "Loading shortcut…",
    reference: { type: "topic", id: "missing" },
  });
});
