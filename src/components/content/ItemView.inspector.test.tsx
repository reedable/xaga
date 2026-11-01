import { act, useEffect, type RefObject } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { ItemView } from "./ItemView";

const mocks = vi.hoisted(() => ({
  state: {
    entryId: "article",
    annotationId: undefined as string | undefined,
    url: "https://example.org/article",
    mimeType: undefined,
  },
  search: undefined as string | undefined,
  resolve: vi.fn(async () => true),
  mounts: vi.fn(),
  openAnnotation: undefined as
    | undefined
    | ((annotation: { id: string; reference: string }) => void),
  replaced: undefined as
    | undefined
    | ((event: { oldId: string; item: { id: string } }) => void),
  removed: undefined as undefined | ((id: string) => void),
}));
vi.mock("../../hooks/useEvents", () => ({
  useEvents: () => ({
    onAnnotationReplaced: (callback: typeof mocks.replaced) => {
      mocks.replaced = callback;
      return () => {};
    },
    onAnnotationRemoved: (callback: typeof mocks.removed) => {
      mocks.removed = callback;
      return () => {};
    },
  }),
}));
vi.mock("../../hooks/useRouter", () => ({
  default: () => ({ state: mocks.state, search: mocks.search }),
}));
vi.mock("../../hooks/useContentLoader", () => ({
  useContentLoader: () => ({
    resolve: mocks.resolve,
    pdfBlobUrl: null,
    blockedUrl: null,
  }),
}));
vi.mock("./ArticleReader", () => ({
  default: function Reader({
    onOpenInspector,
  }: {
    onOpenInspector: (annotation: { id: string; reference: string }) => void;
  }) {
    mocks.openAnnotation = onOpenInspector;
    useEffect(() => {
      mocks.mounts();
    }, []);
    return (
      <div data-reader>
        Saved content
        <iframe title="Article" />
        <button
          onClick={() =>
            onOpenInspector({ id: "tooltip-annotation", reference: "article" })
          }
        >
          Inspect tooltip Annotation
        </button>
      </div>
    );
  },
}));
vi.mock("./Inspector", () => ({
  Inspector: ({
    target,
    view,
    onViewChange,
  }: {
    target: { entryId: string; annotationId?: string };
    view: string;
    onViewChange: (view: string) => void;
  }) => (
    <div data-inspector={target.annotationId ?? target.entryId}>
      <button
        role="tab"
        aria-selected={view === "topics"}
        onClick={() => onViewChange("topics")}
      >
        Topics
      </button>
      <p>{view}</p>
    </div>
  ),
}));
vi.mock("./Toolbar", () => ({
  Toolbar: ({
    inspectorOpener,
    onOpenInspector,
  }: {
    inspectorOpener: RefObject<HTMLButtonElement>;
    onOpenInspector?: () => void;
  }) =>
    onOpenInspector ? (
      <button ref={inspectorOpener} onClick={onOpenInspector}>
        Open Inspector
      </button>
    ) : null,
}));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
const observers = new Map<Element, ResizeObserverCallback>();
vi.stubGlobal(
  "ResizeObserver",
  class {
    callback: ResizeObserverCallback;
    elements = new Set<Element>();
    constructor(callback: ResizeObserverCallback) {
      this.callback = callback;
    }
    observe(element: Element) {
      this.elements.add(element);
      observers.set(element, this.callback);
    }
    unobserve(element: Element) {
      observers.delete(element);
    }
    disconnect() {
      for (const element of this.elements) observers.delete(element);
    }
  }
);
let root: ReturnType<typeof createRoot>;
let container: HTMLDivElement;
beforeEach(async () => {
  vi.clearAllMocks();
  mocks.search = undefined;
  mocks.state = {
    entryId: "article",
    annotationId: undefined,
    url: "https://example.org/article",
    mimeType: undefined,
  };
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
  await act(async () => root.render(<ItemView />));
});
afterEach(async () => {
  await act(async () => root.unmount());
  container.remove();
  observers.clear();
});
async function resize(width: number) {
  const element = container.firstElementChild!;
  await act(async () =>
    observers.get(element)!(
      [{ contentRect: { width } } as ResizeObserverEntry],
      {} as ResizeObserver
    )
  );
}
function opener() {
  return [...container.querySelectorAll("button")].find(
    (button) => button.textContent === "Open Inspector"
  )!;
}

it("retains the reader and selected view through panel/drawer transitions and restores opener focus", async () => {
  const reader = container.querySelector<HTMLElement>("[data-reader]")!;
  reader.scrollTop = 123;
  await act(async () =>
    container.querySelector<HTMLButtonElement>('[role="tab"]')!.click()
  );
  await resize(699);
  await act(async () => {
    opener().focus();
    opener().click();
  });
  expect(document.querySelector('[role="dialog"]')?.textContent).toContain(
    "topics"
  );
  expect(
    document.querySelector('[role="dialog"]')?.contains(document.activeElement)
  ).toBe(true);
  await act(async () =>
    document
      .querySelector<HTMLButtonElement>('[aria-label="Close Inspector"]')!
      .click()
  );
  expect(document.activeElement).toBe(opener());
  await act(async () => opener().click());
  expect(document.querySelector('[role="dialog"]')?.textContent).toContain(
    "topics"
  );
  await resize(700);
  await act(async () => {
    await new Promise<void>((resolve) =>
      requestAnimationFrame(() => resolve())
    );
  });
  expect(container.querySelector("aside")?.textContent).toContain("topics");
  expect(
    container.querySelector("aside")?.contains(document.activeElement)
  ).toBe(true);
  expect(container.querySelector("[data-reader]")).toBe(reader);
  expect(reader.scrollTop).toBe(123);
  expect(mocks.mounts).toHaveBeenCalledOnce();
});

it("targets the selected annotation without changing navigation on Escape dismissal", async () => {
  await resize(600);
  await act(async () => {
    opener().focus();
    opener().click();
  });
  mocks.state = { ...mocks.state, annotationId: "annotation" };
  await act(async () => root.render(<ItemView />));
  expect(
    document
      .querySelector('[role="dialog"] [data-inspector]')
      ?.getAttribute("data-inspector")
  ).toBe("annotation");
  await act(async () =>
    document.activeElement!.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Escape", bubbles: true })
    )
  );
  expect(document.activeElement).toBe(opener());
  expect(mocks.state.annotationId).toBe("annotation");
  expect(mocks.state.entryId).toBe("article");
  expect(mocks.mounts).toHaveBeenCalledOnce();
});

it("opens the shared drawer at wide widths and follows replacement/removal", async () => {
  const reader = container.querySelector("[data-reader]");
  const target = () =>
    document
      .querySelector('[role="dialog"] [data-inspector]')
      ?.getAttribute("data-inspector") ??
    container
      .querySelector("aside [data-inspector]")
      ?.getAttribute("data-inspector");
  await act(async () =>
    [...container.querySelectorAll("button")]
      .find((b) => b.textContent === "Inspect tooltip Annotation")!
      .click()
  );
  expect(target()).toBe("tooltip-annotation");
  expect(document.querySelectorAll("[data-inspector]")).toHaveLength(2);
  expect(document.querySelector('[role="dialog"]')).not.toBeNull();
  expect(
    container
      .querySelector("aside [data-inspector]")
      ?.getAttribute("data-inspector")
  ).toBe("article");
  expect(mocks.state.annotationId).toBeUndefined();
  await act(async () =>
    mocks.replaced!({ oldId: "tooltip-annotation", item: { id: "recolored" } })
  );
  expect(target()).toBe("recolored");
  await act(async () => mocks.removed!("recolored"));
  expect(target()).toBe("article");
  expect(container.querySelector("[data-reader]")).toBe(reader);
  expect(mocks.mounts).toHaveBeenCalledOnce();
});

it("opens the same narrow Inspector from the tooltip and restores reader focus", async () => {
  await resize(600);
  await act(async () =>
    [...container.querySelectorAll("button")]
      .find((b) => b.textContent === "Inspect tooltip Annotation")!
      .click()
  );
  expect(
    document
      .querySelector('[role="dialog"] [data-inspector]')
      ?.getAttribute("data-inspector")
  ).toBe("tooltip-annotation");
  await act(async () =>
    document
      .querySelector<HTMLButtonElement>('[aria-label="Close Inspector"]')!
      .click()
  );
  expect(document.activeElement).toBe(container.querySelector("iframe"));
  expect(mocks.state.annotationId).toBeUndefined();
});

it("ignores a tooltip handoff completed after navigating to another Annotation", async () => {
  const oldHandoff = mocks.openAnnotation!;
  mocks.state = { ...mocks.state, annotationId: "new-selection" };
  await act(async () => root.render(<ItemView />));
  await act(async () =>
    oldHandoff({ id: "old-tooltip", reference: "article" })
  );
  expect(document.querySelector('[role="dialog"]')).toBeNull();
  expect(mocks.state.annotationId).toBe("new-selection");
});

it("keeps Annotation inspection in the drawer when resizing from narrow to wide", async () => {
  await resize(600);
  await act(async () =>
    mocks.openAnnotation!({ id: "annotation", reference: "article" })
  );
  await resize(900);
  expect(
    document
      .querySelector('[role="dialog"] [data-inspector]')
      ?.getAttribute("data-inspector")
  ).toBe("annotation");
  expect(
    container
      .querySelector("aside [data-inspector]")
      ?.getAttribute("data-inspector")
  ).toBe("article");
  await act(async () =>
    document
      .querySelector<HTMLButtonElement>('[aria-label="Close Inspector"]')!
      .click()
  );
  expect(
    container
      .querySelector("aside [data-inspector]")
      ?.getAttribute("data-inspector")
  ).toBe("article");
  expect(document.activeElement).toBe(container.querySelector("iframe"));
});

it("preserves the Article panel and its selected view while the Annotation drawer is open", async () => {
  const panel = container.querySelector("aside")!;
  await act(async () =>
    panel.querySelector<HTMLButtonElement>('[role="tab"]')!.click()
  );
  const before = panel.innerHTML;
  await act(async () =>
    mocks.openAnnotation!({ id: "annotation", reference: "article" })
  );
  expect(container.querySelector("aside")).toBe(panel);
  expect(panel.innerHTML).toBe(before);
  await act(async () =>
    document
      .querySelector<HTMLButtonElement>('[role="dialog"] [role="tab"]')!
      .click()
  );
  expect(panel.innerHTML).toBe(before);
});

it.each([600, 900])(
  "does not open the Inspector on Annotation selection at width %s",
  async (width) => {
    await resize(width);
    mocks.state = { ...mocks.state, annotationId: "selected-annotation" };
    await act(async () => root.render(<ItemView />));
    expect(document.querySelector('[role="dialog"]')).toBeNull();
    expect(mocks.state.annotationId).toBe("selected-annotation");
    await act(async () =>
      mocks.openAnnotation!({ id: "selected-annotation", reference: "article" })
    );
    expect(
      document
        .querySelector('[role="dialog"] [data-inspector]')
        ?.getAttribute("data-inspector")
    ).toBe("selected-annotation");
  }
);

it("does not open the Inspector when initially mounted on an Annotation route", async () => {
  mocks.state = { ...mocks.state, annotationId: "annotation" };
  await act(async () => root.render(<ItemView key="annotation-route" />));
  expect(document.querySelector('[role="dialog"]')).toBeNull();
});


it("keeps the docked Inspector mounted while responsive width hides and shows it", async () => {
  const panel = container.querySelector("aside")!;
  const inspector = panel.querySelector("[data-inspector]")!;
  await resize(699);
  expect(container.querySelector("aside")).toBe(panel);
  expect(panel.getAttribute("aria-hidden")).toBe("true");
  expect(panel.hasAttribute("inert")).toBe(true);
  expect(panel.querySelector("[data-inspector]")).toBe(inspector);
  await resize(700);
  expect(container.querySelector("aside")).toBe(panel);
  expect(panel.getAttribute("aria-hidden")).toBe("false");
  expect(panel.querySelector("[data-inspector]")).toBe(inspector);
});


it("clips the mounted Inspector when Search opens and restores it when Search closes", async () => {
  const panel = container.querySelector("aside")!;
  const inspector = panel.querySelector("[data-inspector]");
  mocks.search = "";
  await act(async () => root.render(<ItemView />));
  expect(panel.getAttribute("aria-hidden")).toBe("true");
  expect(panel.hasAttribute("inert")).toBe(true);
  expect(opener()).toBeDefined();
  expect(panel.querySelector("[data-inspector]")).toBe(inspector);
  mocks.search = undefined;
  await act(async () => root.render(<ItemView />));
  expect(panel.getAttribute("aria-hidden")).toBe("false");
  expect(panel.hasAttribute("inert")).toBe(false);
  expect(panel.querySelector("[data-inspector]")).toBe(inspector);
});
