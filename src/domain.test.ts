import { expect, it } from "vitest";
import {
  isCollection,
  isEntity,
  isManagedCollection,
  isManagedEntity,
  isSite,
  type Feed,
  type Topic,
} from "./domain";
import { ALL_ENTITIES } from "./Shortcuts";
import { emptySite } from "./sites";

const feed: Feed = {
  __kind: "feed",
  id: "feed",
  title: "Feed",
  createdAt: "2026-01-01",
  url: "https://example.org/feed",
  members: [],
  entryCount: 2,
  unreadCount: 1,
  isPinned: false,
};
const topic: Topic = { ...feed, __kind: "topic", id: "topic" };

it("recognizes Feed and Topic as managed collections even when loaded members differ from counts", () => {
  for (const value of [feed, topic]) {
    expect(isCollection(value)).toBe(true);
    expect(isEntity(value)).toBe(true);
    expect(isManagedEntity(value)).toBe(true);
    expect(isManagedCollection(value)).toBe(true);
    expect(isSite(value)).toBe(false);
  }
});

it("recognizes a Site without fabricated timestamps or loaded membership", () => {
  const site = emptySite("example.org");
  expect(isCollection(site)).toBe(true);
  expect(isEntity(site)).toBe(true);
  expect(isManagedEntity(site)).toBe(false);
  expect(isSite(site)).toBe(true);
  expect(isManagedCollection(site)).toBe(false);
  expect(site).not.toHaveProperty("createdAt");
  expect(site).not.toHaveProperty("members");
  expect(site).not.toHaveProperty("hostname");
});

it("requires identity and title for entities and a creation timestamp for managed entities", () => {
  const entity = { id: "entity", title: "Entity" };
  expect(isEntity(entity)).toBe(true);
  expect(isManagedEntity(entity)).toBe(false);
  expect(isManagedEntity({ ...entity, createdAt: "2026-01-01" })).toBe(true);

  for (const value of [
    null,
    undefined,
    "entity",
    {},
    { id: "entity" },
    { title: "Entity" },
    { ...entity, id: 1 },
    { ...entity, title: 1 },
  ]) {
    expect(isEntity(value)).toBe(false);
    expect(isManagedEntity(value)).toBe(false);
  }
  expect(isManagedEntity({ ...entity, createdAt: 1 })).toBe(false);
});

it("does not classify shortcuts or count-bearing articles as collections", () => {
  for (const value of [ALL_ENTITIES, { ...feed, __kind: "entry" }, { ...feed, __kind: undefined }]) {
    expect(isCollection(value)).toBe(false);
    expect(isManagedCollection(value)).toBe(false);
    expect(isSite(value)).toBe(false);
  }
});

it("requires a Site brand, string identity and title, and numeric counts", () => {
  const site = emptySite("example.org");
  for (const value of [
    null,
    { ...site, __kind: undefined },
    { ...site, __kind: "other" },
    { ...site, id: undefined, hostname: "example.org" },
    { ...site, id: 42 },
    { ...site, title: undefined },
    { ...site, entryCount: undefined },
    { ...site, unreadCount: "1" },
  ]) {
    expect(isSite(value)).toBe(false);
    expect(isCollection(value)).toBe(false);
  }
});
