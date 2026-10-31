import { act } from "react";
import type { ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import App from "./App";

const mocks = vi.hoisted(() => ({
  startupState: "loading" as "loading" | "available" | "unavailable",
  platform: {
    onShutdown: vi.fn().mockResolvedValue(() => {}),
    setWindowTitle: vi.fn().mockResolvedValue(undefined),
  },
}));

vi.mock("./hooks/useEmbeddingStartupState", () => ({
  useEmbeddingStartupState: () => mocks.startupState,
}));
vi.mock("./hooks/useSettings", () => ({
  useSettings: () => ({ library: "/tmp/library" }),
}));
vi.mock("./platform/Platform", () => ({ usePlatform: () => mocks.platform }));
vi.mock("./AppLayout", () => ({
  AppLayout: () => <div data-testid="app-layout" />,
}));
vi.mock("./providers/ExpandedListProvider", () => ({
  ExpandedListProvider: ({ children }: { children: ReactNode }) => children,
}));
vi.mock("./providers/AllItemsCountsProvider", () => ({
  AllItemsCountsProvider: ({ children }: { children: ReactNode }) => children,
}));
vi.mock("./providers/FeedsProvider", () => ({
  default: ({ children }: { children: ReactNode }) => children,
}));
vi.mock("./providers/TopicsProvider", () => ({
  TopicsProvider: ({ children }: { children: ReactNode }) => children,
}));
vi.mock("./providers/SitesProvider", () => ({
  SitesProvider: ({
    children,
    enabled,
  }: {
    children: ReactNode;
    enabled: boolean;
  }) => <div data-sites-enabled={String(enabled)}>{children}</div>,
}));

vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);

let container: HTMLDivElement;
let root: ReturnType<typeof createRoot>;

beforeEach(() => {
  vi.clearAllMocks();
  container = document.createElement("div");
  root = createRoot(container);
});

afterEach(async () => {
  await act(async () => root.unmount());
});

async function renderWith(state: typeof mocks.startupState) {
  mocks.startupState = state;
  await act(async () => root.render(<App entryPoint="main" />));
}

it("waits while embedding startup is loading", async () => {
  await renderWith("loading");
  expect(container.querySelector("[data-testid='app-layout']")).toBeNull();
  expect(
    container.querySelector("[data-sites-enabled='false']")
  ).not.toBeNull();
});

it("renders the main application when embeddings are available", async () => {
  await renderWith("available");
  expect(container.querySelector("[data-testid='app-layout']")).not.toBeNull();
  expect(container.querySelector("[data-sites-enabled='true']")).not.toBeNull();
});

it("renders the main application when embeddings are unavailable", async () => {
  await renderWith("unavailable");
  expect(container.querySelector("[data-testid='app-layout']")).not.toBeNull();
  expect(
    container.querySelector("[data-sites-enabled='false']")
  ).not.toBeNull();
});

vi.mock("./providers/PinsProvider", () => ({ default: ({ children }: { children: ReactNode }) => children }));
