import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { useStartupFeedRefresh } from "./useStartupFeedRefresh";
const mocks = vi.hoisted(() => ({
  settings: { settingsLoaded: false, refreshOnStartup: true },
  refresh: vi.fn(),
  platform: { isLibraryReadOnly: vi.fn() },
  sink: { pushError: vi.fn() },
}));
vi.mock("./useSettings", () => ({ useSettings: () => mocks.settings }));
vi.mock("./useFeeds", () => ({
  useFeeds: () => ({ refreshAllFeeds: mocks.refresh }),
}));
vi.mock("../platform/Platform", () => ({ usePlatform: () => mocks.platform }));
vi.mock("./useErrorSink", () => ({ useErrorSink: () => mocks.sink }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let root: ReturnType<typeof createRoot>;
afterEach(async () => {
  await act(() => root.unmount());
  vi.resetAllMocks();
});
it.each([false, true])(
  "waits for settings and mode, then refreshes once only when writable (readOnly=%s)",
  async (readOnly) => {
    mocks.settings.settingsLoaded = false;
    mocks.settings.refreshOnStartup = true;
    mocks.platform.isLibraryReadOnly.mockResolvedValue(readOnly);
    function Host() {
      useStartupFeedRefresh();
      return null;
    }
    root = createRoot(document.createElement("div"));
    await act(() => root.render(<Host />));
    expect(mocks.refresh).not.toHaveBeenCalled();
    mocks.settings.settingsLoaded = true;
    await act(() => root.render(<Host />));
    expect(mocks.refresh).toHaveBeenCalledTimes(readOnly ? 0 : 1);
    await act(() => root.render(<Host />));
    expect(mocks.refresh).toHaveBeenCalledTimes(readOnly ? 0 : 1);
  }
);
