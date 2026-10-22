import { act, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { useSidebarFilter } from "./useSidebarFilter";

vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);

let container: HTMLDivElement;
let root: ReturnType<typeof createRoot>;
let observedFilter = "";
let setObservedFilter: (value: string) => void;

function Observer({ library }: { library: string }) {
  const [filter, setFilter] = useSidebarFilter(library);

  useEffect(() => {
    observedFilter = filter;
    setObservedFilter = setFilter;
  }, [filter, setFilter]);

  return null;
}

beforeEach(() => {
  container = document.createElement("div");
  root = createRoot(container);
  observedFilter = "";
});

afterEach(async () => {
  await act(async () => root.unmount());
});

it("preserves the filter while its owner remains mounted", async () => {
  await act(async () => root.render(<Observer library="one" />));
  await act(async () => setObservedFilter("alpha"));
  await act(async () => root.render(<Observer library="one" />));

  expect(observedFilter).toBe("alpha");
});

it("clears the filter when the active library changes", async () => {
  await act(async () => root.render(<Observer library="one" />));
  await act(async () => setObservedFilter("alpha"));
  await act(async () => root.render(<Observer library="two" />));

  expect(observedFilter).toBe("");
});
