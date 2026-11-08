import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import EditFeedDialog from "./EditFeedDialog";
import EditTopicDialog from "./EditTopicDialog";
const mocks = vi.hoisted(() => ({ readOnly: false, pushError: vi.fn() }));
vi.mock("../../hooks/useLibraryReadOnly", () => ({
  useLibraryReadOnly: () => mocks.readOnly,
}));
vi.mock("../../hooks/useErrorSink", () => ({
  useErrorSink: () => ({ pushError: mocks.pushError }),
}));
vi.mock("../../hooks/useFeeds", () => ({
  useFeeds: () => ({ pinnedFeedIds: [] }),
}));
vi.mock("../../hooks/useTopics", () => ({
  useTopics: () => ({ pinnedTopicIds: [] }),
}));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let root: ReturnType<typeof createRoot>;
afterEach(async () => {
  await act(() => root.unmount());
  vi.clearAllMocks();
});
it.each(["Feed", "Topic"])(
  "disables %s writes but allows closing and inspection",
  async (kind) => {
    mocks.readOnly = true;
    const close = vi.fn();
    const save = vi.fn();
    root = createRoot(document.createElement("div"));
    await act(() =>
      root.render(
        kind === "Feed" ? (
          <EditFeedDialog
            open
            feed={
              {
                id: "feed",
                title: "Notes",
                url: "https://example.org",
              } as never
            }
            onSave={save}
            onAdd={save}
            onDelete={save}
            onClose={close}
          />
        ) : (
          <EditTopicDialog
            open
            topic={{ id: "topic", title: "Notes" } as never}
            onSave={save}
            onDelete={save}
            onClose={close}
          />
        )
      )
    );
    expect(document.body.textContent).toContain("This library is read-only");
    const buttons = Array.from(document.querySelectorAll("button"));
    expect(buttons.find((b) => b.textContent === "Save")?.disabled).toBe(true);
    expect(buttons.find((b) => b.textContent === "Delete")?.disabled).toBe(
      true
    );
    await act(() => buttons.find((b) => b.textContent === "Cancel")!.click());
    expect(close).toHaveBeenCalledOnce();
    expect(save).not.toHaveBeenCalled();
  }
);
it.each(["Feed", "Topic"])(
  "keeps a refused %s save open and reports it",
  async (kind) => {
    mocks.readOnly = false;
    const close = vi.fn();
    const error = {
      code: "library:read-only",
      message: "This library is read-only.",
      detail: "fixture",
    };
    const save = vi.fn().mockRejectedValue(error);
    root = createRoot(document.createElement("div"));
    await act(() =>
      root.render(
        kind === "Feed" ? (
          <EditFeedDialog
            open
            feed={
              {
                id: "feed",
                title: "Notes",
                url: "https://example.org",
              } as never
            }
            onSave={save}
            onAdd={save}
            onDelete={save}
            onClose={close}
          />
        ) : (
          <EditTopicDialog
            open
            topic={{ id: "topic", title: "Notes" } as never}
            onSave={save}
            onDelete={save}
            onClose={close}
          />
        )
      )
    );
    await act(async () => {
      document
        .querySelector("form")!
        .dispatchEvent(
          new Event("submit", { bubbles: true, cancelable: true })
        );
    });
    expect(mocks.pushError).toHaveBeenCalledWith(error);
    expect(close).not.toHaveBeenCalled();
  }
);
