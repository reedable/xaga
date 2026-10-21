import { act, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { SidebarSection, SidebarSortMode } from "./useSidebarSort";
import { useSidebarSort } from "./useSidebarSort";

const storage = vi.hoisted(() => ({
  getViewState: vi.fn(),
  setViewState: vi.fn(),
}));

vi.mock("../storage/viewState", () => storage);
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);

type SortState = readonly [SidebarSortMode, (value: SidebarSortMode) => void];

let captured = new Map<SidebarSection, SortState>();

function Capture({ section }: { section: SidebarSection }) {
  const state = useSidebarSort(section);
  useEffect(() => {
    captured.set(section, state);
  }, [section, state]);
  return null;
}

let root: ReturnType<typeof createRoot>;

beforeEach(() => {
  vi.clearAllMocks();
  captured = new Map();
  storage.getViewState.mockResolvedValue(undefined);
  storage.setViewState.mockResolvedValue(undefined);
  root = createRoot(document.createElement("div"));
});

afterEach(async () => {
  await act(async () => root.unmount());
});

it("defaults each section independently to unread ordering", async () => {
  await act(async () => {
    root.render(
      <>
        <Capture section="shortcuts" />
        <Capture section="topics" />
      </>
    );
  });

  expect(captured.get("shortcuts")?.[0]).toBe("unread");
  expect(captured.get("topics")?.[0]).toBe("unread");
  expect(storage.getViewState).toHaveBeenCalledWith("sidebar.sort.shortcuts");
  expect(storage.getViewState).toHaveBeenCalledWith("sidebar.sort.topics");
});

it("restores a valid saved selection and ignores invalid stored state", async () => {
  storage.getViewState.mockImplementation((key: string) =>
    Promise.resolve(
      key === "sidebar.sort.feeds" ? "alphabetical" : "not-a-sort-mode"
    )
  );

  await act(async () => {
    root.render(
      <>
        <Capture section="feeds" />
        <Capture section="sites" />
      </>
    );
  });

  expect(captured.get("feeds")?.[0]).toBe("alphabetical");
  expect(captured.get("sites")?.[0]).toBe("unread");
});

it("persists a selection under only that section's key", async () => {
  await act(async () => root.render(<Capture section="topics" />));

  await act(async () => captured.get("topics")?.[1]("alphabetical"));

  expect(captured.get("topics")?.[0]).toBe("alphabetical");
  expect(storage.setViewState).toHaveBeenCalledWith(
    "sidebar.sort.topics",
    "alphabetical"
  );
  expect(storage.setViewState).toHaveBeenCalledTimes(1);
});
