import { act, useContext, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import FeedsProvider from "./FeedsProvider";
import FeedsContext, { type FeedsContextType } from "./FeedsContext";
import type {
  BookmarkImportOutcome,
  ExportOutcome,
  Feed,
  ImportOutcome,
} from "../types";

const platform = vi.hoisted(() => ({
  onFeedRefreshed: vi.fn().mockResolvedValue(() => {}),
  onUnreadChanged: vi.fn().mockResolvedValue(() => {}),
  onFeedsImported: vi.fn().mockResolvedValue(() => {}),
  onFeedsExported: vi.fn().mockResolvedValue(() => {}),
  onBookmarksImported: vi.fn().mockResolvedValue(() => {}),
  onBookmarksExported: vi.fn().mockResolvedValue(() => {}),
  listFeeds: vi.fn(),
}));
const sink = vi.hoisted(() => ({ pushError: vi.fn() }));
vi.mock("../platform/Platform", () => ({ usePlatform: () => platform }));
vi.mock("../hooks/useErrorSink", () => ({ useErrorSink: () => sink }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let feeds: FeedsContextType;
function Capture() {
  const value = useContext(FeedsContext)!;
  useEffect(() => {
    feeds = value;
  });
  return null;
}
let root: ReturnType<typeof createRoot>;
beforeEach(async () => {
  vi.clearAllMocks();
  platform.listFeeds.mockResolvedValue([
    { id: "kept", title: "Imported feed", unreadCount: 0 },
  ]);
  root = createRoot(document.createElement("div"));
  await act(async () =>
    root.render(
      <FeedsProvider>
        <Capture />
      </FeedsProvider>
    )
  );
});
afterEach(async () => {
  await act(async () => root.unmount());
});
const feedImported = (outcome: ImportOutcome<Feed>) =>
  platform.onFeedsImported.mock.calls[0][0](outcome);
const bookmarksImported = (outcome: BookmarkImportOutcome) =>
  platform.onBookmarksImported.mock.calls[0][0](outcome);
const bookmarksExported = (outcome: ExportOutcome) =>
  platform.onBookmarksExported.mock.calls[0][0](outcome);

it("reports partial feed failure and refreshes successfully imported feeds", async () => {
  await act(async () =>
    feedImported({ error: null, result: { added: [], skipped: [], failed: 2 } })
  );
  expect(feeds.feeds.map((feed) => feed.id)).toEqual(["kept"]);
  expect(sink.pushError).toHaveBeenCalledWith(
    expect.objectContaining({ message: expect.stringContaining("2 feeds") })
  );
});
it("does not report an error for a successful import", async () => {
  await act(async () =>
    feedImported({ error: null, result: { added: [], skipped: [], failed: 0 } })
  );
  expect(sink.pushError).not.toHaveBeenCalled();
});
it("reports complete feed failure without replacing existing feeds", async () => {
  await act(async () =>
    feedImported({ error: null, result: { added: [], skipped: [], failed: 0 } })
  );
  const error = {
    code: "app:error" as const,
    message: "Import failed",
    detail: "",
  };
  await act(async () => feedImported({ error, result: null }));
  expect(feeds.feeds.map((feed) => feed.id)).toEqual(["kept"]);
  expect(sink.pushError).toHaveBeenCalledWith(error);
  expect(platform.listFeeds).toHaveBeenCalledTimes(1);
});
it("reports partial and complete bookmark failures and export failures", async () => {
  await act(async () =>
    bookmarksImported({
      error: null,
      result: { added: ["kept"], updated: [], failed: 3 },
    })
  );
  expect(sink.pushError).toHaveBeenLastCalledWith(
    expect.objectContaining({ message: expect.stringContaining("3 bookmarks") })
  );
  const error = {
    code: "import:too-large" as const,
    message: "Use a smaller file",
    detail: "",
  };
  await act(async () => bookmarksImported({ error, result: null }));
  expect(sink.pushError).toHaveBeenLastCalledWith(error);
  await act(async () => bookmarksExported({ error }));
  expect(sink.pushError).toHaveBeenLastCalledWith(error);
  sink.pushError.mockClear();
  await act(async () => bookmarksExported({ error: null }));
  await act(async () =>
    bookmarksImported({
      error: null,
      result: { added: [], updated: [], failed: 0 },
    })
  );
  expect(sink.pushError).not.toHaveBeenCalled();
});

vi.mock("./PinsContext", () => ({ usePins: () => ({ pins: [], setPin: vi.fn() }) }));
