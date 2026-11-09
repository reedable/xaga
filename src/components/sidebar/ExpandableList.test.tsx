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

it("disables outgoing rows immediately and restores them when reopened", async () => {
  async function render(isExpanded: boolean) {
    await act(async () => {
      root.render(
        <ExpandableList
          count={1}
          isExpanded={isExpanded}
          label="Topics"
          onClickHeader={vi.fn()}
        >
          <li>
            <button>Everyday discoveries</button>
          </li>
        </ExpandableList>
      );
    });
  }

  await render(true);
  const list = container.querySelector('[aria-label="Topics"]')!;
  expect(list.closest("[inert]")).toBeNull();
  await render(false);
  expect(list.closest('[aria-hidden="true"][inert]')).not.toBeNull();
  // Rows stay mounted for the visual transition but cannot receive input.
  expect(list.querySelector("button")?.textContent).toBe(
    "Everyday discoveries"
  );
  await render(true);
  expect(list.closest("[inert]")).toBeNull();
  expect(list.closest('[aria-hidden="true"]')).toBeNull();
});
