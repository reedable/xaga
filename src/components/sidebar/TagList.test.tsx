import { act, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { MemoryRouter } from "react-router-dom";
import { afterEach, expect, it, vi } from "vitest";
import useRouter from "../../hooks/useRouter";
import { TagsContext } from "../../providers/TagsContext";
import TagList from "./TagList";

vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let root: ReturnType<typeof createRoot>;
let container: HTMLDivElement;
let router: ReturnType<typeof useRouter>;
function Host() {
  const current = useRouter();
  useEffect(() => { router = current; }, [current]);
  const tags = [{ id: "a", name: "Research" }, { id: "b", name: "Work" }];
  return <TagsContext.Provider value={{ tags, hasLoaded: true, unreadCount: 3 }}>
    <TagList tags={tags} isExpanded onClickHeader={() => {}} />
  </TagsContext.Provider>;
}
afterEach(async () => { await act(async () => root.unmount()); container.remove(); });
it("toggles the shared tag intersection by keyboard and displays its actual unread count", async () => {
  container = document.createElement("div"); document.body.append(container);
  root = createRoot(container);
  await act(async () => root.render(<MemoryRouter initialEntries={[{ pathname: "/", state: { tagIds: ["a"], filter: "saved" } }]}><Host /></MemoryRouter>));
  const choices = () => [...container.querySelectorAll<HTMLElement>('[role="checkbox"]')];
  expect(choices()[0].getAttribute("aria-checked")).toBe("true");
  expect(container.textContent).toContain("Matching unread: 3");
  await act(async () => {
    choices()[1].focus();
    choices()[1].dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
  });
  expect(router.state).toEqual({ tagIds: ["a", "b"], filter: "saved" });
  await act(async () => choices()[0].click());
  expect(router.state.tagIds).toEqual(["b"]);
  await act(async () => choices()[1].click());
  expect(router.state.tagIds).toEqual([]);
});
