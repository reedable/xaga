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
  resolve: vi.fn(async () => true),
  mounts: vi.fn(),
}));
vi.mock("../../hooks/useRouter", () => ({
  default: () => ({ state: mocks.state }),
}));
vi.mock("../../hooks/useContentLoader", () => ({
  useContentLoader: () => ({
    resolve: mocks.resolve,
    pdfBlobUrl: null,
    blockedUrl: null,
  }),
}));
vi.mock("./ArticleReader", () => ({
  default: function Reader() {
    useEffect(() => {
      mocks.mounts();
    }, []);
    return <div data-reader>Saved content</div>;
  },
}));
vi.mock("./ArticleInspector", () => ({
  ArticleInspector: ({
    articleId,
    view,
    onViewChange,
  }: {
    articleId: string;
    view: string;
    onViewChange: (view: string) => void;
  }) => (
    <div data-inspector={articleId}>
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

it("keeps Inspector targeting the parent article during annotation navigation and Escape dismissal", async () => {
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
  ).toBe("article");
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
