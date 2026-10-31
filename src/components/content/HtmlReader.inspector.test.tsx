import { act, useState } from "react";
import { flushSync } from "react-dom";
import { Drawer } from "@mui/material";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { HtmlReader } from "./HtmlReader";
import { HIGHLIGHT_CLICKED, SELECTION_MADE } from "../../text/buildEntryHtml";
import type { Annotation } from "../../types";

const mocks = vi.hoisted(() => ({
  save: vi.fn(),
  open: vi.fn(),
  remove: vi.fn(),
  pushError: vi.fn(),
  readOnly: false,
  removed: undefined as undefined | ((id: string) => void),
  replaced: undefined as
    | undefined
    | ((event: { oldId: string; item: { id: string; color: string } }) => void),
}));
const events = {
  onAnnotationReplaced: (callback: typeof mocks.replaced) => {
    mocks.replaced = callback;
    return () => {};
  },
  onFind: () => () => {},
  onAnnotationRemoved: (callback: (id: string) => void) => {
    mocks.removed = callback;
    return () => {};
  },
};
const sink = { pushError: mocks.pushError };
vi.mock("../../hooks/useEvents", () => ({ useEvents: () => events }));
vi.mock("../../hooks/useAppearance", () => ({
  useAppearance: () => ({ themeMode: "light" }),
}));
vi.mock("../../hooks/useLibraryReadOnly", () => ({
  useLibraryReadOnly: () => mocks.readOnly,
}));
vi.mock("../../hooks/useErrorSink", () => ({ useErrorSink: () => sink }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
URL.createObjectURL = vi.fn(() => "about:blank");
URL.revokeObjectURL = vi.fn();
const annotation = {
  __kind: "annotation",
  id: "annotation",
  reference: "article",
  color: "blue",
  text: ["Passage"],
  occurrence: [0],
  content: "Saved note",
} as Annotation;
let container: HTMLDivElement;
let root: ReturnType<typeof createRoot>;
const button = (label: string) =>
  container.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`)!;
async function message(data: object) {
  await act(async () =>
    window.dispatchEvent(
      new MessageEvent("message", {
        source: container.querySelector("iframe")!.contentWindow,
        data,
      })
    )
  );
}
beforeEach(async () => {
  vi.clearAllMocks();
  mocks.readOnly = false;
  mocks.save.mockResolvedValue(undefined);
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
  await act(async () =>
    root.render(
      <HtmlReader
        html="<p>Passage</p>"
        entryId="article"
        annotations={[annotation]}
        onSaveAnnotation={mocks.save}
        onRemoveAnnotation={mocks.remove}
        onOpenInspector={mocks.open}
      />
    )
  );
});
afterEach(async () => {
  await act(async () => root.unmount());
  container.remove();
});
async function highlight() {
  await message({
    type: HIGHLIGHT_CLICKED,
    annotationId: "annotation",
    rect: { left: 0, top: 0, height: 10, width: 20 },
  });
}
it("saves the tooltip before handing its Annotation to the Inspector", async () => {
  let finish!: () => void;
  mocks.save.mockImplementationOnce(
    () =>
      new Promise<void>((resolve) => {
        finish = resolve;
      })
  );
  await highlight();
  await act(async () => button("Open Inspector").click());
  expect(mocks.open).not.toHaveBeenCalled();
  expect(mocks.save).toHaveBeenCalledWith(
    expect.objectContaining({
      id: "annotation",
      reference: "article",
      content: "Saved note",
    })
  );
  await act(async () => finish());
  expect(mocks.open).toHaveBeenCalledWith(
    {
      id: "annotation",
      reference: "article",
    },
    expect.any(HTMLButtonElement)
  );
  expect(button("Open Inspector")).not.toBeNull();
});
it("keeps a failed save visible and does not open the Inspector", async () => {
  const error = new Error("write failed");
  mocks.save.mockRejectedValueOnce(error);
  await highlight();
  await act(async () => button("Open Inspector").click());
  expect(mocks.pushError).toHaveBeenCalledWith(error);
  expect(mocks.open).not.toHaveBeenCalled();
  expect(button("Open Inspector")).not.toBeNull();
});
it("offers inspection only after a fresh selection has a color", async () => {
  await message({
    type: SELECTION_MADE,
    texts: ["Passage"],
    occurrences: [0],
    rect: { left: 0, top: 0, height: 10, width: 20 },
  });
  expect(button("Open Inspector")).toBeNull();
  await act(async () => button("blue").click());
  const id = mocks.save.mock.calls[0][0].id;
  await act(async () => button("Open Inspector").click());
  expect(mocks.open).toHaveBeenCalledWith(
    { id, reference: "article" },
    expect.any(HTMLButtonElement)
  );
});
it("inspects an existing Annotation read-only without saving it", async () => {
  mocks.readOnly = true;
  await highlight();
  await act(async () => button("Open Inspector").click());
  expect(mocks.save).not.toHaveBeenCalled();
  expect(mocks.open).toHaveBeenCalledWith(
    {
      id: "annotation",
      reference: "article",
    },
    expect.any(HTMLButtonElement)
  );
});

it("keeps the tooltip through drawer interaction and resumes dismissal after closing", async () => {
  await highlight();
  const opener = button("Open Inspector");
  await act(async () => opener.click());
  const render = (inspectedAnnotationId?: string) =>
    root.render(
      <HtmlReader
        html="<p>Passage</p>"
        entryId="article"
        annotations={[annotation]}
        onSaveAnnotation={mocks.save}
        onRemoveAnnotation={mocks.remove}
        onOpenInspector={mocks.open}
        inspectedAnnotationId={inspectedAnnotationId}
      />
    );
  await act(async () => render("annotation"));
  expect(button("Open Inspector")).toBe(opener);
  const outside = document.createElement("button");
  document.body.append(outside);
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 1));
    outside.click();
    window.dispatchEvent(new Event("blur"));
  });
  expect(button("Open Inspector")).toBe(opener);
  expect(opener.closest("[inert]")).not.toBeNull();
  await act(async () => render());
  expect(opener.closest("[inert]")).toBeNull();
  await act(async () => outside.click());
  expect(button("Open Inspector")).toBeNull();
  outside.remove();
});
it("removes the retained tooltip when its Annotation is deleted", async () => {
  await highlight();
  await act(async () => button("Open Inspector").click());
  const saves = mocks.save.mock.calls.length;
  await act(async () => mocks.removed!("annotation"));
  expect(button("Open Inspector")).toBeNull();
  expect(mocks.save).toHaveBeenCalledTimes(saves);
});

it("follows replacement identity in the retained tooltip", async () => {
  await highlight();
  await act(async () =>
    mocks.replaced!({
      oldId: "annotation",
      item: { id: "recolored", color: "red" },
    })
  );
  await act(async () => button("Open Inspector").click());
  expect(mocks.open).toHaveBeenLastCalledWith(
    { id: "recolored", reference: "article" },
    expect.any(HTMLButtonElement)
  );
});

it.each(["Close", "backdrop", "Escape"])(
  "preserves the tooltip when the drawer closes via %s",
  async (method) => {
    function Host() {
      const [open, setOpen] = useState(false);
      const close = () => flushSync(() => setOpen(false));
      return (
        <>
          <HtmlReader
            html="<p>Passage</p>"
            entryId="article"
            annotations={[annotation]}
            onSaveAnnotation={mocks.save}
            onOpenInspector={() => setOpen(true)}
            inspectedAnnotationId={open ? "annotation" : undefined}
          />
          <Drawer
            data-inspector-drawer
            open={open}
            onClose={close}
            transitionDuration={0}
          >
            <button aria-label="Close test drawer" onClick={close}>
              Close
            </button>
          </Drawer>
        </>
      );
    }
    await act(async () => root.render(<Host />));
    await highlight();
    const opener = button("Open Inspector");
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 1));
      opener.click();
    });
    const close = document.querySelector<HTMLElement>(
      method === "Close"
        ? '[aria-label="Close test drawer"]'
        : ".MuiBackdrop-root"
    )!;
    await act(async () => {
      if (method === "Escape")
        document.activeElement!.dispatchEvent(
          new KeyboardEvent("keydown", { key: "Escape", bubbles: true })
        );
      else close.click();
    });
    expect(button("Open Inspector")).toBe(opener);
    expect(opener.closest("[inert]")).toBeNull();
  }
);
