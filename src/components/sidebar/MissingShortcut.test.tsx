import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import MissingShortcut from "./MissingShortcut";
import TopicsContext from "../../providers/TopicsContext";
const mocks = vi.hoisted(() => ({
  readOnly: false,
  setPin: vi.fn().mockResolvedValue(undefined),
  refreshPins: vi.fn().mockResolvedValue(undefined),
  pushError: vi.fn(),
  readTopic: vi.fn(),
  listTopics: vi.fn(),
  listFeeds: vi.fn(),
  readEntry: vi.fn(),
}));
vi.mock("../../providers/PinsContext", () => ({ usePins: () => mocks }));
vi.mock("../../platform/Platform", () => ({ usePlatform: () => mocks }));
vi.mock("../../hooks/useLibraryReadOnly", () => ({
  useLibraryReadOnly: () => mocks.readOnly,
}));
vi.mock("../../hooks/useErrorSink", () => ({ useErrorSink: () => mocks }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let root: ReturnType<typeof createRoot>;
function mount() {
  const container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
  return container;
}
function button(label: string) {
  return [...document.querySelectorAll<HTMLButtonElement>("button")]
    .find((button) => button.textContent === label)!;
}
afterEach(async () => {
  if (root) await act(async () => root.unmount());
  document.body.innerHTML = "";
  mocks.readOnly = false;
  vi.clearAllMocks();
});
it("keeps initial loading as a skeleton and hides target IDs in the compact error row", async () => {
  const container = mount();
  const render = (hasLoaded: boolean) => (
    <TopicsContext.Provider value={{ setMembership: vi.fn(), getMemberships: (_id: string, ids: string[]) => ids,
      topics: [], pinnedTopicIds: ["pending"], hasLoaded,
      setTopics: vi.fn(), setTopicPinned: vi.fn(),
    }}>
      <MissingShortcut target={{ type: "topic", id: "pending" }} />
    </TopicsContext.Provider>
  );
  await act(async () => root.render(render(false)));
  expect(container.querySelector('[aria-busy="true"]')).not.toBeNull();
  expect(container.querySelector("button")).toBeNull();
  await act(async () => root.render(render(true)));
  expect(container.textContent).toBe("Shortcut unavailable");
  expect(container.textContent).not.toContain("pending");
  expect(container.querySelector('[aria-busy="true"]')).toBeNull();
});
it("opens details with keyboard-focusable actions and removes only the shortcut", async () => {
  const container = mount();
  const target = { type: "topic" as const, id: "lost" };
  await act(async () => root.render(<MissingShortcut target={target} />));
  const row = container.querySelector<HTMLElement>('[role="button"]')!;
  await act(async () => row.focus());
  expect(document.activeElement).toBe(row);
  await act(async () => row.click());
  expect(document.querySelector('[role="dialog"]')!.textContent).toContain("topic: lost");
  expect(document.querySelector('[role="dialog"]')!.textContent).toContain("We can’t find this topic.");
  await act(async () => button("Remove shortcut").click());
  expect(mocks.setPin).toHaveBeenCalledWith(target, false);
});
it("allows retry in read-only libraries and displays the safe read error", async () => {
  const container = mount();
  mocks.readOnly = true;
  mocks.readTopic.mockRejectedValue({
    code: "topic:malformed", message: "The Topic file could not be read.", detail: "SECRET",
  });
  await act(async () => root.render(<MissingShortcut target={{ type: "topic", id: "broken" }} />));
  await act(async () => container.querySelector<HTMLElement>('[role="button"]')!.click());
  expect(button("Remove shortcut").disabled).toBe(true);
  expect(button("Retry").disabled).toBe(false);
  await act(async () => button("Retry").click());
  expect(mocks.readTopic).toHaveBeenCalledWith(expect.any(AbortSignal), "broken");
  expect(document.body.textContent).toContain("The Topic file could not be read.");
  expect(document.body.textContent).not.toContain("SECRET");
  expect(mocks.setPin).not.toHaveBeenCalled();
});
it("refreshes shared Topics on successful retry", async () => {
  const container = mount();
  const setTopics = vi.fn();
  const results = [{ id: "restored" }];
  mocks.readTopic.mockResolvedValue(results[0]);
  mocks.listTopics.mockResolvedValue(results);
  await act(async () => root.render(
    <TopicsContext.Provider value={{ setMembership: vi.fn(), getMemberships: (_id: string, ids: string[]) => ids, topics: [], hasLoaded: true, pinnedTopicIds: [], setTopics, setTopicPinned: vi.fn() }}>
      <MissingShortcut target={{ type: "topic", id: "restored" }} />
    </TopicsContext.Provider>
  ));
  await act(async () => container.querySelector<HTMLElement>('[role="button"]')!.click());
  await act(async () => button("Retry").click());
  expect(setTopics).toHaveBeenCalledWith(results);
});
it("opens shortcut details from the responsive header", async () => {
  const { default: MissingShortcutsButton } = await import("./MissingShortcutsButton");
  const container = mount();
  await act(async () => root.render(<MissingShortcutsButton targets={[{ type: "feed", id: "lost-feed" }]} />));
  await act(async () => container.querySelector("button")!.click());
  await act(async () => document.querySelector<HTMLElement>('[role="dialog"] [role="button"]')!.click());
  expect(document.body.textContent).toContain("feed: lost-feed");
  expect(button("Retry")).toBeTruthy();
});
