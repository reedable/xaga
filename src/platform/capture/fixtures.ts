import gardenImage from "./garden.png?inline";
import waterImage from "./water.png?inline";
import woodImage from "./wood.png?inline";

export const captureImages: Readonly<Record<string, string>> = {
  "../images/capture/garden.png": gardenImage,
  "../images/capture/water.png": waterImage,
  "../images/capture/wood.png": woodImage,
};
import type { PinNode } from "../../pins";
import type { Article, Feed, Topic, Tag } from "../../types";

export const captureDate = "2026-01-15T12:00:00.000Z";
const captureFeedUrl = "https://fieldnotes.example/feed.xml";
export const additionalFeedUrl = "https://workshop.example/feed.xml";

const imageAltByArticleId: Record<string, string> = {
  garden: "A flower garden surrounded by trees and a wooden fence",
  water: "A stream flowing over stones through a mossy forest",
  wood: "A woodworking workshop with workbenches, timber, and wall-mounted tools",
};

export function createFixtures() {
  const articles: Article[] = [
    ["garden", "A garden for every season", "fieldnotes.example", true],
    ["water", "Following the water", "fieldnotes.example", true],
    ["wood", "Making room for a workshop", "workshop.example", false],
  ].map(([id, title, hostname, unread]) => ({
    __kind: "entry",
    id: String(id),
    title: String(title),
    url: `https://${hostname}/${id}`,
    createdAt: captureDate,
    publishedAt: captureDate,
    isPinned: false,
    isUnread: Boolean(unread),
    description: "Small observations, practical ideas, and time to explore.",
    content: `Small changes give us a new way to notice familiar places.\n\n![${imageAltByArticleId[String(id)]}](../images/capture/${id}.png)\n\nBegin with one question and make time to follow it. Record what you discover, then return to it with fresh eyes.\n\n## Try it yourself\n\nChoose a place you know well. Spend a few minutes observing it and write down three details you had not noticed before.`,
    annotations: [],
  }));
  articles[2].color = "yellow";
  articles[2].annotations.push({
    __kind: "annotation",
    id: "observation",
    reference: "wood",
    title: "Begin with one question",
    text: ["Begin with one question"],
    occurrence: [0],
    color: "yellow",
    content: "A useful starting point for my next project.",
    createdAt: captureDate,
  });
  const feeds: Feed[] = [
    {
      __kind: "feed",
      id: "fieldnotes",
      title: "Field notes",
      url: captureFeedUrl,
      createdAt: captureDate,
      isPinned: false,
      entryCount: 2,
      unreadCount: 2,
      members: articles.slice(0, 2).map(({ id }) => ({ __kind: "entry", id })),
    },
  ];
  const topics: Topic[] = [
    {
      __kind: "topic",
      id: "observations",
      title: "Everyday discoveries",
      description: "Ideas worth revisiting",
      createdAt: captureDate,
      isPinned: false,
      members: [{ __kind: "entry", id: "garden" }],
      entryCount: 1,
      unreadCount: 1,
    },
  ];
  const tags: Tag[] = [
    { id: "projects", name: "Projects" },
    { id: "outdoors", name: "Outdoors" },
  ];
  return {
    articles,
    feeds,
    topics,
    tags,
    assignments: {
      garden: ["outdoors"],
      wood: ["projects"],
      observation: ["projects"],
    } as Record<string, string[]>,
  };
}

function nestedShortcutGroups(
  depth: number,
  prefix: string,
  members: PinNode[]
): PinNode {
  let node: PinNode = {
    type: "group",
    id: `${prefix}-${depth}`,
    title: `${prefix} level ${depth}`,
    children: members,
  };
  for (let level = depth - 1; level > 0; level--)
    node = {
      type: "group",
      id: `${prefix}-${level}`,
      title: `${prefix} level ${level}`,
      children: [node],
    };
  return node;
}

/** Long lists and valid nested groups for height-responsive sidebar workflows. */
export function createSidebarHeightFixtures() {
  const template = createFixtures().topics[0];
  const topics = Array.from({ length: 80 }, (_, index) => ({
    ...template,
    id: `height-topic-${index}`,
    title: `Collection ${String(index).padStart(2, "0")}`,
    members: [],
    entryCount: 0,
    unreadCount: 0,
  }));
  const targets = topics.map((topic) => ({
    type: "topic" as const,
    id: topic.id,
  }));
  return {
    topics,
    pins: [
      nestedShortcutGroups(5, "Five", targets.slice(0, 40)),
      nestedShortcutGroups(24, "Deep", targets.slice(40)),
    ],
  };
}

/** Sibling boundaries, variable heading heights, counts, and deep stored trees. */
export function createStickyShortcutFixtures() {
  const { topics } = createSidebarHeightFixtures();
  const targets = topics.map((topic) => ({
    type: "topic" as const,
    id: topic.id,
  }));
  const pins: PinNode[] = [
    {
      type: "group",
      id: "research",
      title: "Research",
      children: [
        {
          type: "group",
          id: "background",
          title: "Background with a long title that wraps over several lines",
          children: [
            {
              type: "group",
              id: "sources",
              title: "Sources",
              children: [
                { type: "topic", id: "observations" },
                ...targets.slice(0, 20),
              ],
            },
          ],
        },
        {
          type: "group",
          id: "methods",
          title: "Methods",
          children: targets.slice(20, 40),
        },
      ],
    },
    {
      type: "group",
      id: "work",
      title: "Work",
      children: targets.slice(40, 60),
    },
    nestedShortcutGroups(5, "Five", targets.slice(60, 70)),
    nestedShortcutGroups(24, "Deep", targets.slice(70)),
  ];
  return { topics, pins };
}
