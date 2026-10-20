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
    members: [{ __kind: "annotation", id: "annotation-1" }],
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
    { __kind: "annotation", id: "annotation-1" },
  ]);
  expect(invokeMock).toHaveBeenLastCalledWith("create_topic_with_member", {
    title: "Reading",
    description: "Worth revisiting",
    threshold: 0.75,
    item: { type: "annotation", id: "annotation-1", reference: "entry-1" },
  });
});

it("reads the embedding startup state from the native boundary", async () => {
  invokeMock.mockResolvedValueOnce("unavailable");
  await expect(new TauriPlatform().embeddingStartupState()).resolves.toBe(
    "unavailable"
  );
  expect(invokeMock).toHaveBeenLastCalledWith("embedding_startup_state");
});

it("maps All Items counts from the native boundary", async () => {
  invokeMock.mockResolvedValueOnce({ entry_count: 12, unread_count: 4 });
  await expect(new TauriPlatform().getAllItemsCounts()).resolves.toEqual({
    entryCount: 12,
    unreadCount: 4,
  });
  expect(invokeMock).toHaveBeenLastCalledWith("get_all_items_counts");
});

it("keeps All Items counts unavailable when the cache is unavailable", async () => {
  invokeMock.mockResolvedValueOnce(null);
  await expect(new TauriPlatform().getAllItemsCounts()).resolves.toBeNull();
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
