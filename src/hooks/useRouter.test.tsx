import { act, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { MemoryRouter } from "react-router-dom";
import { afterEach, expect, it, vi } from "vitest";
import useRouter from "./useRouter";
import { ALL_ENTITIES } from "../Shortcuts";
import type { Entry, Feed, Topic, NavigationState } from "../types";
import { emptySite } from "../sites";

vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let root: ReturnType<typeof createRoot> | undefined;
let container: HTMLDivElement;
let router: ReturnType<typeof useRouter>;
function Host() {
  const value = useRouter();
  useEffect(() => {
    router = value;
  }, [value]);
  return null;
}
async function render(state: NavigationState) {
  container = document.createElement("div");
  root = createRoot(container);
  await act(async () =>
    root!.render(
      <MemoryRouter initialEntries={[{ pathname: "/", state }]}>
        <Host />
      </MemoryRouter>
    )
  );
}
afterEach(async () => {
  await act(async () => root?.unmount());
});

it("keeps the resolved Article URL when selecting another Annotation in that Article", async () => {
  await render({
    entryId: "article",
    annotationId: "first",
    url: "https://example.org/article",
    mimeType: "text/html",
  });
  await act(async () =>
    router.selectAnnotation({ id: "second", reference: "article" })
  );
  expect(router.state).toMatchObject({
    entryId: "article",
    annotationId: "second",
    url: "https://example.org/article",
    mimeType: "text/html",
  });
});

it("clears the previous Article URL when selecting an Annotation in a different Article", async () => {
  await render({
    entryId: "article",
    url: "https://example.org/article",
    mimeType: "text/html",
  });
  await act(async () =>
    router.selectAnnotation({ id: "annotation", reference: "other" })
  );
  expect(router.state.entryId).toBe("other");
  expect(router.state.annotationId).toBe("annotation");
  expect(router.state.url).toBeUndefined();
  expect(router.state.mimeType).toBeUndefined();
});

it("uses the URL already carried by an Annotation row when changing Articles", async () => {
  await render({ entryId: "article", url: "https://example.org/article" });
  await act(async () =>
    router.selectAnnotation({
      id: "annotation",
      reference: "other",
      url: "https://example.org/other",
    })
  );
  expect(router.state).toMatchObject({
    entryId: "other",
    annotationId: "annotation",
    url: "https://example.org/other",
  });
});

it("keeps tag selection while reading and filtering, and leaves it on collection navigation", async () => {
  await render({ tagIds: ["b", "a"], filter: "saved" });
  await act(async () => router.selectTags(["b", "a", "a"]));
  expect(router.state.tagIds).toEqual(["a", "b"]);
  await act(async () => router.selectEntity({ __kind: "entry", id: "entry", title: "Entry", url: "https://example.org", createdAt: "now", isPinned: false } as Entry));
  expect(router.state.tagIds).toEqual(["a", "b"]);
  await act(async () => router.selectFilter("all"));
  expect(router.state.tagIds).toEqual(["a", "b"]);
  await act(async () => router.selectTags([]));
  expect(router.state).toEqual({ tagIds: [], filter: "all" });
  await act(async () => router.selectShortcut(ALL_ENTITIES));
  expect(router.state.tagIds).toBeUndefined();
});

const feed: Feed = {
  __kind: "feed", id: "feed", title: "Feed", createdAt: "2026-01-01",
  url: "https://example.org/feed", members: [], isPinned: false, entryCount: 2, unreadCount: 1,
};
const topic: Topic = { ...feed, __kind: "topic", id: "topic" };

it.each([
  [feed, { feedId: feed.id }],
  [topic, { topicId: topic.id }],
  [emptySite("example.org"), { siteHostname: "example.org" }],
] as const)("routes $0.__kind through common collection selection with sticky and explicit filters", async (collection, scope) => {
  await render({ articleScopeId: "article", entryId: "article", filter: "saved" });
  await act(async () => router.selectCollection(collection));
  expect(router.state).toEqual({ ...scope, filter: "saved" });
  const selected = router.state;
  await act(async () => router.selectEntity(collection));
  expect(router.state).toBe(selected);
  await act(async () => router.selectCollection(collection, "all"));
  expect(router.state).toEqual({ ...scope, filter: "all" });
});

it("routes built-in shortcuts separately and keeps article selection in its collection", async () => {
  await render({ siteHostname: "example.org", filter: "saved" });
  const entry: Entry = {
    __kind: "entry", id: "entry", title: "Entry", createdAt: "2026-01-01",
    url: "https://example.org/article", isPinned: true,
  };
  await act(async () => router.selectEntity(entry));
  expect(router.state).toMatchObject({ siteHostname: "example.org", entryId: entry.id });
  expect(router.state.articleScopeId).toBeUndefined();
  await act(async () => router.selectArticleScope(entry));
  expect(router.state).toEqual({ articleScopeId: entry.id, entryId: entry.id, url: entry.url });
  await act(async () => router.selectEntity(ALL_ENTITIES));
  expect(router.state).toEqual({ shortcutId: ALL_ENTITIES.id, filter: undefined });
});
