import { act, useContext, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { Site } from "../types";
import SitesContext from "./SitesContext";
import { SitesProvider } from "./SitesProvider";

const platform = vi.hoisted(() => ({
  listSites: vi.fn(),
  onEmbeddingsChanged: vi.fn(),
  onUnreadChanged: vi.fn(),
}));
vi.mock("../platform/Platform", () => ({ usePlatform: () => platform }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);

let sites: Site[] = [];
function Capture() {
  const value = useContext(SitesContext);
  useEffect(() => {
    sites = value?.sites ?? [];
  }, [value]);
  return null;
}

let root: ReturnType<typeof createRoot>;
beforeEach(async () => {
  vi.useFakeTimers();
  vi.clearAllMocks();
  sites = [];
  platform.listSites.mockResolvedValue([
    {
      id: "example.com",
      title: "example.com",
      hostname: "example.com",
      entryCount: 1,
      unreadCount: 1,
    },
  ]);
  platform.onEmbeddingsChanged.mockResolvedValue(() => {});
  platform.onUnreadChanged.mockResolvedValue(() => {});
  root = createRoot(document.createElement("div"));
  await act(async () => {
    root.render(
      <SitesProvider>
        <Capture />
      </SitesProvider>
    );
  });
});

afterEach(async () => {
  await act(async () => root.unmount());
  vi.useRealTimers();
});

it("loads one-entry Sites and refreshes after live changes", async () => {
  expect(sites).toHaveLength(1);
  const embeddingsChanged = platform.onEmbeddingsChanged.mock.calls[0][0];
  const unreadChanged = platform.onUnreadChanged.mock.calls[0][0];

  await act(async () => {
    embeddingsChanged();
    unreadChanged({});
    await vi.advanceTimersByTimeAsync(400);
  });

  expect(platform.listSites).toHaveBeenCalledTimes(2);
});
