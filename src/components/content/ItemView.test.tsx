import { act, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { ItemView } from "./ItemView";

const mocks = vi.hoisted(() => ({
  state: {
    url: "https://example.org/document",
    entryId: "entry",
    mimeType: undefined as string | undefined,
  },
  resolve: vi.fn(),
  readerMounted: vi.fn(),
}));
vi.mock("../../hooks/useRouter", () => ({
  default: () => ({ state: mocks.state }),
}));
vi.mock("../../hooks/useContentLoader", () => ({
  useContentLoader: () => ({
    resolve: mocks.resolve,
    pdfBlobUrl: "blob:validated",
    blockedUrl: null,
  }),
}));
vi.mock("@mui/material", () => ({
  Typography: ({ children }: { children: React.ReactNode }) => (
    <span>{children}</span>
  ),
}));
vi.mock("./ArticleReader", () => ({
  default: function SavedReader() {
    useEffect(() => {
      mocks.readerMounted();
    }, []);
    return <div>HTML reader</div>;
  },
}));
vi.mock("./PdfReader", () => ({ PdfReader: () => <div>PDF reader</div> }));
vi.mock("./Toolbar", () => ({
  Toolbar: ({
    reload,
    reloading,
  }: {
    reload: () => void;
    reloading: boolean;
  }) => <button onClick={reload}>{reloading ? "busy" : "refresh"}</button>,
}));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let root: ReturnType<typeof createRoot>;
let container: HTMLDivElement;
let finish: (result?: boolean) => void;
beforeEach(() => {
  vi.clearAllMocks();
  mocks.state = {
    url: "https://example.org/document",
    entryId: "entry",
    mimeType: undefined,
  };
  mocks.resolve.mockImplementation(
    () =>
      new Promise<boolean | undefined>((resolve) => {
        finish = resolve;
      })
  );
  container = document.createElement("div");
  root = createRoot(container);
});
afterEach(async () => {
  await act(() => root.unmount());
});
it("waits for URL classification before mounting a saved entry's HTML reader", async () => {
  await act(() => root.render(<ItemView />));
  expect(container.textContent).toContain("Loading…");
  mocks.state = { ...mocks.state, mimeType: "application/pdf" };
  await act(async () => {
    finish();
    root.render(<ItemView />);
  });
  expect(container.textContent).toContain("PDF reader");
  expect(mocks.resolve).toHaveBeenCalledTimes(1);
});
it("resolves a history selection even when its route already carries PDF MIME", async () => {
  mocks.state.mimeType = "application/pdf";
  await act(() => root.render(<ItemView />));
  expect(mocks.resolve).toHaveBeenCalledWith(mocks.state.url);
  expect(container.textContent).toContain("Loading…");
  await act(async () => {
    finish();
  });
  expect(container.textContent).toContain("PDF reader");
});

it("ignores repeated activations and clears progress after a failed reload", async () => {
  await act(() => root.render(<ItemView />));
  await act(async () => {
    finish();
  });
  const button = container.querySelector("button")!;
  await act(() => {
    button.click();
    button.click();
  });
  expect(mocks.resolve).toHaveBeenCalledTimes(2);
  expect(button.textContent).toBe("busy");
  expect(container.textContent).toContain("HTML reader");
  await act(async () => {
    finish();
  });
  expect(button.textContent).toBe("refresh");
  expect(container.textContent).toContain("HTML reader");
  expect(mocks.readerMounted).toHaveBeenCalledOnce();
});

it("does not let an old reload clear a newer article's progress", async () => {
  await act(() => root.render(<ItemView />));
  await act(async () => {
    finish();
  });
  await act(() => container.querySelector("button")!.click());
  const oldFinish = finish;
  mocks.state = {
    ...mocks.state,
    url: "https://example.org/other",
    entryId: "other",
  };
  await act(() => root.render(<ItemView />));
  await act(async () => {
    finish();
  });
  await act(() => container.querySelector("button")!.click());
  await act(async () => {
    oldFinish();
  });
  expect(container.querySelector("button")!.textContent).toBe("busy");
  await act(async () => {
    finish();
  });
  expect(container.querySelector("button")!.textContent).toBe("refresh");
});

it("updates the reader only after a successful reload and permits a later retry", async () => {
  await act(() => root.render(<ItemView />));
  await act(async () => {
    finish(true);
  });
  expect(mocks.readerMounted).toHaveBeenCalledOnce();
  await act(() => container.querySelector("button")!.click());
  expect(mocks.readerMounted).toHaveBeenCalledOnce();
  await act(async () => {
    finish(true);
  });
  expect(mocks.readerMounted).toHaveBeenCalledTimes(2);
  await act(() => container.querySelector("button")!.click());
  expect(mocks.resolve).toHaveBeenCalledTimes(3);
  await act(async () => {
    finish(false);
  });
  expect(mocks.readerMounted).toHaveBeenCalledTimes(2);
});
