import { beforeEach, describe, expect, it, vi } from "vitest";

// Pins the openUrl scheme allowlist: only web/mail schemes reach the system
// opener; file:/javascript:/data: and custom schemes must be refused before
// the plugin is called.
const { openUrlMock } = vi.hoisted(() => ({ openUrlMock: vi.fn() }));
const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
const { closeRequested } = vi.hoisted(() => ({
  closeRequested: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ label: "main", onCloseRequested: closeRequested }),
}));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: openUrlMock }));
// TauriPlatform registers a module-load event listener; no-op it under test.
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

import { TauriPlatform } from "./TauriPlatform";

it("creates a topic with an existing member through one native command", async () => {
  invokeMock.mockResolvedValueOnce({
    id: "topic-1",
    title: "Reading",
    created_at: "2026-10-20T00:00:00Z",
    updated_at: null,
    description: "Worth revisiting",
    threshold: 0.75,
    entry_count: 1,
    unread_count: 1,
    is_pinned: false,
    members: [
      { __kind: "annotation", id: "annotation-1", articleId: "entry-1" },
    ],
  });
  const member = {
    __kind: "annotation" as const,
    id: "annotation-1",
    title: "Highlight",
    createdAt: "2026-10-20T00:00:00Z",
    reference: "entry-1",
    text: ["Highlight"],
    occurrence: [0],
    color: "yellow",
    content: "",
  };

  const result = await new TauriPlatform().createTopicWithMember(
    new AbortController().signal,
    { title: "Reading", description: "Worth revisiting", threshold: 0.75 },
    member
  );

  expect(result.id).toBe("topic-1");
  expect(result.members).toEqual([
    { __kind: "annotation", id: "annotation-1", articleId: "entry-1" },
  ]);
  expect(invokeMock).toHaveBeenLastCalledWith("create_topic_with_member", {
    title: "Reading",
    description: "Worth revisiting",
    threshold: 0.75,
    entity: { __kind: "annotation", id: "annotation-1", articleId: "entry-1" },
  });
});

it("reads the embedding startup state from the native boundary", async () => {
  invokeMock.mockResolvedValueOnce("unavailable");
  await expect(new TauriPlatform().embeddingStartupState()).resolves.toBe(
    "unavailable"
  );
  expect(invokeMock).toHaveBeenLastCalledWith("embedding_startup_state");
});

it("maps All Entities counts from the native boundary", async () => {
  invokeMock.mockResolvedValueOnce({ entry_count: 12, unread_count: 4 });
  await expect(new TauriPlatform().getAllEntitiesCounts()).resolves.toEqual({
    entryCount: 12,
    unreadCount: 4,
  });
  expect(invokeMock).toHaveBeenLastCalledWith("get_all_entities_counts");
});

it("maps Site summaries and sends Site filters through the native boundary", async () => {
  invokeMock.mockResolvedValueOnce([
    { hostname: "example.com", entry_count: 2, unread_count: 1 },
  ]);
  const platform = new TauriPlatform();
  await expect(
    platform.listSites(new AbortController().signal)
  ).resolves.toEqual([
    {
      id: "example.com",
      title: "example.com",
      __kind: "site",
      entryCount: 2,
      unreadCount: 1,
    },
  ]);

  invokeMock.mockResolvedValueOnce([]);
  await platform.listSiteEntries(
    new AbortController().signal,
    "example.com",
    "saved"
  );
  expect(invokeMock).toHaveBeenLastCalledWith("list_site_entries", {
    hostname: "example.com",
    entityFilter: "saved",
  });
});

it("keeps All Entities counts unavailable when the cache is unavailable", async () => {
  invokeMock.mockResolvedValueOnce(null);
  await expect(new TauriPlatform().getAllEntitiesCounts()).resolves.toBeNull();
});

it("preserves native download evidence when reading diagnostics", async () => {
  const download = {
    loaderLabel: "loader-0",
    sourceUrl: "https://example.org/",
    currentUrl: "https://example.org/",
    downloadUrl: "https://example.org/favicon.ico",
    destination: "/Downloads/favicon.ico",
    success: null,
  };
  invokeMock.mockResolvedValueOnce({
    id: "attempt",
    startedAtMs: 1,
    mode: "refresh",
    outcome: "succeeded",
    events: [
      {
        elapsedMs: 10,
        source: "native",
        step: "download_requested",
        outcome: "warning",
        code: "article:download-rejected",
        detail: "",
        download,
      },
    ],
    droppedEvents: 0,
  });
  const result = await new TauriPlatform().getArticleDiagnostic("attempt");
  expect(result?.events[0].download).toEqual(download);
});

describe("TauriPlatform.openUrl scheme allowlist", () => {
  const platform = new TauriPlatform();

  beforeEach(() => openUrlMock.mockClear());

  it.each(["http://example.com/", "https://example.com/", "mailto:a@b.com"])(
    "opens allowed scheme: %s",
    async (url) => {
      await platform.openUrl(url);
      expect(openUrlMock).toHaveBeenCalledWith(url);
    }
  );

  it.each([
    "file:///etc/passwd",
    "javascript:alert(1)",
    "data:text/html,<script>alert(1)</script>",
  ])("refuses disallowed scheme: %s", async (url) => {
    await platform.openUrl(url);
    expect(openUrlMock).not.toHaveBeenCalled();
  });
});

it("deduplicates native errors delivered during startup backlog drain", async () => {
  const { listen } = await import("@tauri-apps/api/event");
  const stop = vi.fn();
  const error = {
    code: "proxy:blocked",
    message: "Remote content was blocked.",
    detail: "main",
    diagnosticId: "network-1",
  };
  vi.mocked(listen).mockImplementationOnce(async (_name, handler) => {
    handler({ payload: error } as never);
    return stop;
  });
  invokeMock.mockResolvedValueOnce([error]);
  const deliver = vi.fn();
  const unlisten = await new TauriPlatform().onError(deliver);
  expect(vi.mocked(listen)).toHaveBeenLastCalledWith(
    "app:error",
    expect.any(Function),
    { target: { kind: "WebviewWindow", label: "main" } }
  );
  expect(invokeMock).toHaveBeenLastCalledWith("take_ui_errors");
  expect(deliver).toHaveBeenCalledExactlyOnceWith(error);
  unlisten();
  expect(stop).toHaveBeenCalledOnce();
});

it("awaits pending edits before deciding whether the window may close", async () => {
  closeRequested.mockClear();
  let finish!: (saved: boolean) => void;
  const saved = new Promise<boolean>((resolve) => {
    finish = resolve;
  });
  await new TauriPlatform().onWindowCloseRequested(() => saved);
  const event = { preventDefault: vi.fn() };
  let completed = false;
  const closing = closeRequested.mock.calls[0][0](event).then(() => {
    completed = true;
  });
  await Promise.resolve();
  expect(completed).toBe(false);
  finish(false);
  await closing;
  expect(event.preventDefault).toHaveBeenCalledOnce();
});

it("allows closing after edits have been saved", async () => {
  closeRequested.mockClear();
  await new TauriPlatform().onWindowCloseRequested(async () => true);
  const event = { preventDefault: vi.fn() };
  await closeRequested.mock.calls[0][0](event);
  expect(event.preventDefault).not.toHaveBeenCalled();
});

it("preserves article pins across pin commands, discovery and side-effect-free scope reads", async () => {
  const payload = {
    id: "article",
    title: "Pinned",
    created_at: "2026-10-29",
    updated_at: null,
    published_at: null,
    description: null,
    url: "https://example.org",
    is_unread: true,
    is_pinned: true,
  };
  const platform = new TauriPlatform();
  invokeMock.mockResolvedValueOnce(payload);
  expect((await platform.setEntryPinned("article", true)).isPinned).toBe(true);
  expect(invokeMock).toHaveBeenLastCalledWith("set_entry_pinned", {
    id: "article",
    pinned: true,
  });
  invokeMock.mockResolvedValueOnce([payload]);
  expect(
    (await platform.listPinnedEntries(new AbortController().signal))[0].isPinned
  ).toBe(true);
  invokeMock.mockResolvedValueOnce({
    ...payload,
    content: "Body",
    annotations: [],
    favicon: null,
  });
  expect(
    (await platform.readArticleScope(new AbortController().signal, "article"))
      .isUnread
  ).toBe(true);
  expect(invokeMock).toHaveBeenLastCalledWith("read_article_scope", {
    id: "article",
  });
});

it("carries bookmark colors through commands, list payloads, and events", async () => {
  const payload = {
    id: "marked",
    title: "Marked",
    created_at: "2026-10-31",
    updated_at: null,
    published_at: null,
    description: null,
    url: "https://example.org",
    is_unread: false,
    color: "violet",
  };
  const platform = new TauriPlatform();
  invokeMock.mockResolvedValueOnce(payload);
  const updated = await platform.addBookmark(
    new AbortController().signal,
    "marked",
    "violet"
  );
  expect(invokeMock).toHaveBeenLastCalledWith("add_bookmark", {
    id: "marked",
    color: "violet",
  });
  expect(updated.color).toBe("violet");
  expect(updated).not.toHaveProperty("isBookmarked");
  invokeMock.mockResolvedValueOnce([payload]);
  const entities = await platform.listAllEntities(
    new AbortController().signal,
    "saved"
  );
  expect(entities[0]).toMatchObject({ id: "marked", color: "violet" });
  const listener = vi.fn();
  await platform.onBookmarkChanged(listener);
  const { listen } = await import("@tauri-apps/api/event");
  const handler = vi
    .mocked(listen)
    .mock.calls.findLast(([name]) => name === "bookmark:changed")![1];
  for (const color of ["blue", "green", null]) {
    handler({
      event: "bookmark:changed",
      id: 0,
      payload: { ...payload, color },
    });
    expect(listener).toHaveBeenLastCalledWith(
      expect.objectContaining({ id: "marked", color: color ?? undefined })
    );
  }
});

it("carries Site pin hostnames through discovery and generic mutation commands", async () => {
  const platform = new TauriPlatform();
  const target = { type: "site" as const, hostname: "www.example.com" };
  invokeMock.mockResolvedValueOnce({
    pins: [target, { type: "entry", id: "article" }],
  });
  expect(await platform.listPins(new AbortController().signal)).toEqual([
    target,
    { type: "entry", id: "article" },
  ]);
  invokeMock.mockResolvedValueOnce(undefined);
  await platform.setPin(target, true);
  expect(invokeMock).toHaveBeenLastCalledWith("set_pin", {
    target,
    pinned: true,
  });
  invokeMock.mockResolvedValueOnce({
    pins: [{ type: "site", id: "not-a-hostname-field" }],
  });
  await expect(
    platform.listPins(new AbortController().signal)
  ).rejects.toThrow();
});

it("creates and reads tag identities through the native boundary", async () => {
  const platform = new TauriPlatform();
  const tag = { id: "46fa4a27-1ddf-448a-a5e6-95691a29c602", name: "Research" };
  invokeMock.mockResolvedValueOnce(tag);
  expect(await platform.createTag(" RESEARCH ")).toEqual(tag);
  expect(invokeMock).toHaveBeenLastCalledWith("create_tag", {
    name: " RESEARCH ",
  });
  invokeMock.mockResolvedValueOnce([tag]);
  expect(await platform.listTags(new AbortController().signal)).toEqual([tag]);
  expect(invokeMock).toHaveBeenLastCalledWith("list_tags");
  invokeMock.mockResolvedValueOnce([{ ...tag, id: "research" }]);
  await expect(
    platform.listTags(new AbortController().signal)
  ).rejects.toThrow();
  const controller = new AbortController();
  controller.abort();
  invokeMock.mockResolvedValueOnce([tag]);
  await expect(platform.listTags(controller.signal)).rejects.toThrow();
});

it("delivers committed tag creation payloads and retains listener cleanup", async () => {
  const { listen } = await import("@tauri-apps/api/event");
  const cleanup = vi.fn();
  vi.mocked(listen).mockResolvedValueOnce(cleanup);
  const listener = vi.fn();
  const result = await new TauriPlatform().onTagCreated(listener);
  const handler = vi
    .mocked(listen)
    .mock.calls.findLast(([name]) => name === "tag:created")![1];
  const tag = { id: "46fa4a27-1ddf-448a-a5e6-95691a29c602", name: "Research" };
  handler({ event: "tag:created", id: 0, payload: tag });
  expect(listener).toHaveBeenCalledWith(tag);
  expect(result).toBe(cleanup);
});

it("carries an Entry assignment delta through commands and notifications", async () => {
  const platform = new TauriPlatform();
  const tagId = "46fa4a27-1ddf-448a-a5e6-95691a29c602";
  const change = { entryId: "article", tagId, assigned: true };
  invokeMock.mockResolvedValueOnce([tagId]);
  expect(
    await platform.readEntryTags(new AbortController().signal, "article")
  ).toEqual([tagId]);
  expect(invokeMock).toHaveBeenLastCalledWith("read_entry_tags", {
    id: "article",
  });
  invokeMock.mockResolvedValueOnce(change);
  expect(await platform.setEntryTag("article", tagId, true)).toEqual(change);
  expect(invokeMock).toHaveBeenLastCalledWith("set_entry_tag", {
    id: "article",
    tagId,
    assigned: true,
  });
  const { listen } = await import("@tauri-apps/api/event");
  const callback = vi.fn();
  await platform.onEntryTagsChanged(callback);
  const handler = vi
    .mocked(listen)
    .mock.calls.findLast(([name]) => name === "entry-tags:changed")![1];
  handler({ event: "entry-tags:changed", id: 0, payload: change });
  expect(callback).toHaveBeenCalledWith(change);
});

it("queries tag intersections and counts over the native read boundary", async () => {
  const platform = new TauriPlatform();
  const ids = [
    "46fa4a27-1ddf-448a-a5e6-95691a29c602",
    "46fa4a27-1ddf-448a-a5e6-95691a29c603",
  ];
  invokeMock.mockResolvedValueOnce([]);
  expect(
    await platform.listTagEntries(new AbortController().signal, ids, "saved")
  ).toEqual([]);
  expect(invokeMock).toHaveBeenLastCalledWith("list_tag_entries", {
    tagIds: ids,
    entityFilter: "saved",
  });
  invokeMock.mockResolvedValueOnce(3);
  expect(await platform.countTagUnread(new AbortController().signal, ids)).toBe(
    3
  );
  expect(invokeMock).toHaveBeenLastCalledWith("count_tag_unread", {
    tagIds: ids,
  });
  const controller = new AbortController();
  controller.abort();
  invokeMock.mockResolvedValueOnce([]);
  await expect(
    platform.listTagEntries(controller.signal, [], "all")
  ).rejects.toThrow();
});

it("carries tag rename status and typed definition notifications", async () => {
  const platform = new TauriPlatform();
  const id = "46fa4a27-1ddf-448a-a5e6-95691a29c602";
  const snapshot = {
    tags: [{ id, name: "Changed" }],
    renaming: [id],
    deleting: [],
    merging: [],
  };
  invokeMock.mockResolvedValueOnce(snapshot);
  expect(await platform.renameTag(id, "Changed")).toEqual(snapshot);
  expect(invokeMock).toHaveBeenLastCalledWith("rename_tag", {
    id,
    name: "Changed",
  });
  invokeMock.mockResolvedValueOnce(snapshot);
  expect(await platform.readTagStatus(new AbortController().signal)).toEqual(
    snapshot
  );
  invokeMock.mockResolvedValueOnce({
    ...snapshot,
    deleting: [id],
    merging: [],
  });
  expect(await platform.deleteTag(id)).toEqual({
    ...snapshot,
    deleting: [id],
    merging: [],
  });
  expect(invokeMock).toHaveBeenLastCalledWith("delete_tag", { id });
  const { listen } = await import("@tauri-apps/api/event");
  const callback = vi.fn();
  await platform.onTagsChanged(callback);
  const handler = vi
    .mocked(listen)
    .mock.calls.findLast(([name]) => name === "tags:changed")![1];
  handler({ event: "tags:changed", id: 0, payload: snapshot });
  expect(callback).toHaveBeenCalledWith(snapshot);
});

it("sends explicit merge intent and validates the surviving identity snapshot", async () => {
  const snapshot = {
    tags: [{ id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa", name: "Work" }],
    renaming: [],
    deleting: [],
    merging: [
      {
        sourceId: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
        destinationId: "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
        survivorId: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
        destinationName: "Work",
      },
    ],
  };
  invokeMock.mockResolvedValueOnce(snapshot);
  await expect(
    new TauriPlatform().mergeTags(
      "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
      "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb"
    )
  ).resolves.toEqual(snapshot);
  expect(invokeMock).toHaveBeenLastCalledWith("merge_tags", {
    sourceId: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
    destinationId: "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
  });
});

it("parses hierarchical membership and forwards typed organization through the native boundary", async () => {
  const platform = new TauriPlatform();
  const pins = [
    {
      type: "group",
      id: "g",
      title: "Research",
      children: [{ type: "entry", id: "article" }],
    },
  ];
  invokeMock.mockResolvedValueOnce({ pins });
  expect(await platform.listPins(new AbortController().signal)).toEqual(pins);
  const edit = {
    action: "move" as const,
    node: { type: "entry" as const, id: "article" },
    parent: "g",
  };
  await platform.organizePins(edit);
  expect(invokeMock).toHaveBeenLastCalledWith("organize_pins", { edit });
  const listener = vi.fn();
  const stop = await platform.onPinChanged(listener);
  const { listen } = await import("@tauri-apps/api/event");
  const handler = vi
    .mocked(listen)
    .mock.calls.findLast(([name]) => name === "pins:organized")![1];
  handler({ event: "pins:organized", id: 0, payload: edit });
  expect(listener).toHaveBeenCalledWith({ edit });
  stop();
});

it("validates the shared sidebar sort and delivers committed native changes", async () => {
  const platform = new TauriPlatform();
  invokeMock.mockResolvedValueOnce("alphabetical");
  await expect(platform.getSidebarSort()).resolves.toBe("alphabetical");
  invokeMock.mockResolvedValueOnce("invalid");
  await expect(platform.getSidebarSort()).rejects.toThrow();
  await platform.setSidebarSort("unread");
  expect(invokeMock).toHaveBeenLastCalledWith("set_sidebar_sort", {
    value: "unread",
  });
  const listener = vi.fn();
  await platform.onSidebarSortChanged(listener);
  const { listen } = await import("@tauri-apps/api/event");
  const handler = vi
    .mocked(listen)
    .mock.calls.findLast(([name]) => name === "settings:sidebar-sort")![1];
  handler({ event: "settings:sidebar-sort", id: 0, payload: "alphabetical" });
  expect(listener).toHaveBeenCalledExactlyOnceWith("alphabetical");
  handler({ event: "settings:sidebar-sort", id: 0, payload: "invalid" });
  expect(listener).toHaveBeenCalledTimes(1);
});

it("decodes typed per-feed refresh failures and successful empty passes", async () => {
  const failure = {
    id: "notes",
    title: "Field notes",
    code: "feed:http",
    message: "Couldn't reach this feed.",
  };
  invokeMock.mockResolvedValueOnce({ feeds: [], failures: [failure] });
  expect(
    await new TauriPlatform().refreshAllFeeds(new AbortController().signal)
  ).toEqual({ feeds: [], failures: [failure] });
  expect(invokeMock).toHaveBeenLastCalledWith("refresh_all_feeds", {});
  invokeMock.mockResolvedValueOnce({ feeds: [], failures: [] });
  expect(
    await new TauriPlatform().refreshAllFeeds(new AbortController().signal)
  ).toEqual({ feeds: [], failures: [] });
});

it("transports fetch limits as exact decimal strings without numeric conversion", async () => {
  const platform = new TauriPlatform();
  const settings = {
    refererAllowed: false,
    gzipAllowed: false,
    httpAllowed: false,
    connectNonstandardPortsAllowed: false,
    maxRedirects: "3",
    loaderTimeoutSecs: "18446744073709551615",
    feedTimeoutSecs: "0",
    feedMaxBodyBytes: "9007199254740993",
  };
  invokeMock.mockResolvedValueOnce(settings);
  expect(await platform.getFetchSettings()).toEqual(settings);
  expect(invokeMock).toHaveBeenLastCalledWith("get_fetch_settings");
  invokeMock.mockResolvedValueOnce(undefined);
  await platform.setFetchSetting({
    setting: "feedMaxBodyBytes",
    value: "9007199254740993",
  });
  expect(invokeMock).toHaveBeenLastCalledWith("set_fetch_setting", {
    change: { setting: "feedMaxBodyBytes", value: "9007199254740993" },
  });
});
