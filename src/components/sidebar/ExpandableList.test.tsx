import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import ExpandableList from "./ExpandableList";

vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);

let container: HTMLDivElement;
let root: ReturnType<typeof createRoot>;

beforeEach(() => {
  container = document.createElement("div");
  root = createRoot(container);
});

afterEach(async () => {
  await act(async () => root.unmount());
});

it("shows the matching collection count, including zero", async () => {
  await act(async () => {
    root.render(
      <ExpandableList
        count={0}
        isExpanded={false}
        label="Sites"
        onClickHeader={vi.fn()}
      />
    );
  });

  expect(container.querySelector("button")?.textContent).toContain("Sites (0)");
});
