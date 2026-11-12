import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import ExpandableList from "./ExpandableList";
vi.mock("react-virtuoso", () => import("../../test/mockVirtuoso"));

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
          <button>Everyday discoveries</button>
        </ExpandableList>
      );
    });
  }

  await render(true);
  const list = container.querySelector('[role="list"]')!;
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

it("keeps keyed row state when sorting and exposes positions in the complete list", async () => {
  async function render(ids: string[]) {
    await act(async () => {
      root.render(
        <ExpandableList
          count={ids.length}
          isExpanded
          label="Topics"
          onClickHeader={vi.fn()}
        >
          {ids.map((id) => (
            <input key={id} aria-label={id} defaultValue={id} />
          ))}
        </ExpandableList>
      );
    });
  }
  await render(["alpha", "beta"]);
  const alpha = container.querySelector<HTMLInputElement>(
    '[aria-label="alpha"]'
  )!;
  alpha.value = "edited";
  await render(["beta", "alpha"]);
  expect(container.querySelector('[aria-label="alpha"]')).toBe(alpha);
  expect(alpha.value).toBe("edited");
  expect(
    alpha.closest('[role="listitem"]')?.getAttribute("aria-posinset")
  ).toBe("2");
  expect(alpha.closest('[role="listitem"]')?.getAttribute("aria-setsize")).toBe(
    "2"
  );
  await render([]);
  expect(container.querySelector("[data-virtuoso-scroller]")).toBeNull();
  await render(["beta"]);
  expect(container.querySelector('[aria-label="beta"]')).not.toBeNull();
});

it("allows a nested tree to retain its own rendering", async () => {
  await act(async () =>
    root.render(
      <ExpandableList
        count={1}
        isExpanded
        label="Shortcuts"
        virtualized={false}
        onClickHeader={vi.fn()}
      >
        <li>
          <button>Nested shortcut</button>
        </li>
      </ExpandableList>
    )
  );
  expect(container.querySelector("[data-virtuoso-scroller]")).toBeNull();
  expect(container.querySelector("ul li button")?.textContent).toBe(
    "Nested shortcut"
  );
});
