import { act } from "react";
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
vi.mock("./ArticleReader", () => ({ default: () => <div>HTML reader</div> }));
vi.mock("./PdfReader", () => ({ PdfReader: () => <div>PDF reader</div> }));
vi.mock("./Toolbar", () => ({ Toolbar: () => null }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let root: ReturnType<typeof createRoot>;
let container: HTMLDivElement;
let finish: () => void;
beforeEach(() => {
  vi.clearAllMocks();
  mocks.state = {
    url: "https://example.org/document",
    entryId: "entry",
    mimeType: undefined,
  };
  mocks.resolve.mockImplementation(
    () =>
      new Promise<void>((resolve) => {
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
  expect(container.textContent).toBe("Loading…");
  mocks.state = { ...mocks.state, mimeType: "application/pdf" };
  await act(async () => {
    finish();
    root.render(<ItemView />);
  });
  expect(container.textContent).toBe("PDF reader");
  expect(mocks.resolve).toHaveBeenCalledTimes(1);
});
it("resolves a history selection even when its route already carries PDF MIME", async () => {
  mocks.state.mimeType = "application/pdf";
  await act(() => root.render(<ItemView />));
  expect(mocks.resolve).toHaveBeenCalledWith(mocks.state.url);
  expect(container.textContent).toBe("Loading…");
  await act(async () => { finish(); });
  expect(container.textContent).toBe("PDF reader");
});
