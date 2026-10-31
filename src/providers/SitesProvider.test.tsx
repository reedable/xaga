import { act, useContext, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { Site } from "../types";
import SitesContext from "./SitesContext";
import { SitesProvider } from "./SitesProvider";

const pinState = vi.hoisted(() => ({
  pins: [] as import("../pins").PinTarget[],
}));
vi.mock("./PinsContext", () => ({ usePins: () => pinState }));

const platform = vi.hoisted(() => ({
  listSites: vi.fn(),
  onEmbeddingsChanged: vi.fn(),
  onUnreadChanged: vi.fn(),
}));
vi.mock("../platform/Platform", () => ({ usePlatform: () => platform }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);

let sites: Site[] = [];
let pinnedSites: Site[] = [];
let refreshSites: () => Promise<void>;
function Capture() {
  const value = useContext(SitesContext);
  useEffect(() => {
    sites = value?.sites ?? [];
    pinnedSites = value?.pinnedSites ?? [];
    refreshSites = value!.refreshSites;
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

it("retains empty pinned Sites, updates counts, and removes only the pin projection", async () => {
  pinState.pins = [
    { type: "site", hostname: "example.com" },
    { type: "site", hostname: "empty.example" },
  ];
  await act(async () =>
    root.render(
      <SitesProvider>
        <Capture />
      </SitesProvider>
    )
  );
  expect(
    pinnedSites.map((site) => [
      site.hostname,
      site.entryCount,
      site.unreadCount,
    ])
  ).toEqual([
    ["example.com", 1, 1],
    ["empty.example", 0, 0],
  ]);
  platform.listSites.mockResolvedValue([]);
  await act(async () => refreshSites());
  expect(
    pinnedSites.every((site) => site.entryCount === 0 && site.unreadCount === 0)
  ).toBe(true);
  expect(pinState.pins).toHaveLength(2);
  pinState.pins = [];
  await act(async () =>
    root.render(
      <SitesProvider>
        <Capture />
      </SitesProvider>
    )
  );
  expect(pinnedSites).toEqual([]);
});
