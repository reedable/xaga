import { act } from "react";
import { createRoot } from "react-dom/client";
import { beforeEach, afterEach, expect, it, vi } from "vitest";
import type {
  AnnotationTagChange,
  EntryTagChange,
  Tag,
  TagSnapshot,
} from "../../domain";
import { EntityTags } from "./EntityTags";

const mocks = vi.hoisted(() => ({
  readOnly: false,
  readTagStatus: vi.fn(),
  readEntryTags: vi.fn(),
  readAnnotationTags: vi.fn(),
  setAnnotationTag: vi.fn(),
  onAnnotationTagsChanged: vi.fn(),
  createTag: vi.fn(),
  setEntryTag: vi.fn(),
  onTagsChanged: vi.fn(),
  onTagCreated: vi.fn(),
  onEntryTagsChanged: vi.fn(),
  pushError: vi.fn(),
}));
vi.mock("../../platform/Platform", () => ({ usePlatform: () => mocks }));
const sink = { pushError: mocks.pushError };
vi.mock("../../hooks/useErrorSink", () => ({ useErrorSink: () => sink }));
vi.mock("../../hooks/useLibraryReadOnly", () => ({
  useLibraryReadOnly: () => mocks.readOnly,
}));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
const research = { id: "research", name: "Research" };
const work = { id: "work", name: "Work" };
let container: HTMLDivElement;
let root: ReturnType<typeof createRoot>;
let changeListener: (change: EntryTagChange) => void;
let annotationListener: (change: AnnotationTagChange) => void;
let statusListener: (snapshot: TagSnapshot) => void;
let createdListener: (tag: Tag) => void;
let cleanup: ReturnType<typeof vi.fn>;
beforeEach(() => {
  vi.clearAllMocks();
  mocks.readOnly = false;
  mocks.readTagStatus.mockResolvedValue({
    tags: [research, work],
    renaming: [],
    deleting: [],
    merging: [],
  });
  mocks.readEntryTags.mockResolvedValue([research.id]);
  mocks.readAnnotationTags.mockResolvedValue([research.id]);
  mocks.setAnnotationTag.mockImplementation(
    async (entryId, annotationId, tagId, assigned) => ({
      entryId,
      annotationId,
      tagId,
      assigned,
    })
  );
  mocks.onAnnotationTagsChanged.mockImplementation(async (listener) => {
    annotationListener = listener;
    return cleanup;
  });

  mocks.setEntryTag.mockImplementation(async (entryId, tagId, assigned) => ({
    entryId,
    tagId,
    assigned,
  }));
  mocks.createTag.mockResolvedValue({ id: "new", name: "New" });
  cleanup = vi.fn();
  mocks.onTagsChanged.mockImplementation(async (listener) => {
    statusListener = listener;
    return cleanup;
  });
  mocks.onTagCreated.mockImplementation(async (listener) => {
    createdListener = listener;
    return cleanup;
  });
  mocks.onEntryTagsChanged.mockImplementation(async (listener) => {
    changeListener = listener;
    return cleanup;
  });
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
});
afterEach(async () => {
  await act(async () => root.unmount());
  container.remove();
});
async function render(entryId = "article") {
  await act(async () =>
    root.render(<EntityTags key={entryId} entryId={entryId} />)
  );
}
const chip = (name: string) =>
  [...container.querySelectorAll<HTMLButtonElement>("button")].find(
    (b) => b.textContent === name
  )!;
async function search(value: string) {
  const input = container.querySelector("input")!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(
      HTMLInputElement.prototype,
      "value"
    )!.set!.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
}
it("adds and removes immediately while failed writes retain selection", async () => {
  await render();
  expect(chip("Research").getAttribute("aria-pressed")).toBe("true");
  expect(chip("Work").getAttribute("aria-pressed")).toBe("false");
  await act(async () => chip("Work").click());
  expect(mocks.setEntryTag).toHaveBeenCalledWith("article", "work", true);
  expect(chip("Work").getAttribute("aria-pressed")).toBe("true");
  await act(async () => chip("Research").click());
  expect(mocks.setEntryTag).toHaveBeenLastCalledWith(
    "article",
    "research",
    false
  );
  expect(chip("Research").getAttribute("aria-pressed")).toBe("false");
  mocks.setEntryTag.mockRejectedValueOnce(new Error("write failed"));
  await act(async () => chip("Work").click());
  expect(chip("Work").getAttribute("aria-pressed")).toBe("true");
  expect(mocks.pushError).toHaveBeenCalledOnce();
});
it("filters and creates a definition then assigns it without a Save action", async () => {
  await render();
  await search(" RESEARCH ");
  expect(chip("Research")).toBeTruthy();
  expect(chip("Work")).toBeUndefined();
  expect(container.querySelector('[data-testid="create-tag"]')).toBeNull();
  await search("New");
  await act(async () => chip("Create “New”").click());
  expect(mocks.createTag).toHaveBeenCalledWith("New");
  expect(mocks.setEntryTag).toHaveBeenCalledWith("article", "new", true);
  expect(chip("New").getAttribute("aria-pressed")).toBe("true");
});
it("shows selection read-only and receives changes from another Inspector", async () => {
  mocks.readOnly = true;
  await render();
  expect(chip("Research").disabled).toBe(true);
  await search("New");
  expect(chip("Create “New”").disabled).toBe(true);
  await search("");
  await act(async () => {
    changeListener({ entryId: "another", tagId: "work", assigned: true });
  });
  expect(chip("Work").getAttribute("aria-pressed")).toBe("false");
  await act(async () => {
    changeListener({ entryId: "article", tagId: "work", assigned: true });
    createdListener({ id: "third", name: "Third" });
  });
  expect(chip("Work").getAttribute("aria-pressed")).toBe("true");
  expect(chip("Third")).toBeTruthy();
  expect(mocks.setEntryTag).not.toHaveBeenCalled();
});
it("ignores late results after navigating to another Entry and cleans up listeners", async () => {
  let finish!: (ids: string[]) => void;
  mocks.readEntryTags.mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      })
  );
  await render("old");
  await render("new");
  await act(async () => finish(["work"]));
  expect(chip("Research").getAttribute("aria-pressed")).toBe("true");
  expect(chip("Work").getAttribute("aria-pressed")).toBe("false");
  expect(cleanup).toHaveBeenCalledTimes(3);
});
it("preserves a committed event that arrives during initial loading", async () => {
  let finish!: (ids: string[]) => void;
  mocks.readEntryTags.mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      })
  );
  await render();
  await act(async () => {
    changeListener({ entryId: "article", tagId: "work", assigned: true });
    finish(["research"]);
  });
  expect(chip("Work").getAttribute("aria-pressed")).toBe("true");
});

it("matches registry lowercase semantics without equating distinct spellings", async () => {
  mocks.readTagStatus.mockResolvedValue({
    tags: [
      { id: "german", name: "straße" },
      { id: "greek", name: "ΟΣ" },
    ],
    renaming: [],
    deleting: [],
    merging: [],
  });
  mocks.readEntryTags.mockResolvedValue([]);
  await render();
  await search("strasse");
  expect(chip("Create “strasse”")).toBeTruthy();
  await search("οσ");
  expect(chip("ΟΣ")).toBeTruthy();
  expect(container.querySelector('[data-testid="create-tag"]')).toBeNull();
});

it("retains a committed rename over an older definition load", async () => {
  let finish!: (snapshot: TagSnapshot) => void;
  mocks.readTagStatus.mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      })
  );
  await render();
  await act(async () => {
    statusListener({
      tags: [{ ...research, name: "Renamed" }],
      renaming: [research.id],
      deleting: [],
      merging: [],
    });
    finish({ tags: [research, work], renaming: [], deleting: [], merging: [] });
  });
  expect(chip("Renamed")).toBeTruthy();
  expect(chip("Research")).toBeUndefined();
  expect(chip("Renamed").getAttribute("aria-pressed")).toBe("true");
});

it("blocks assignment to deleting tags from initial status and live updates", async () => {
  mocks.readTagStatus.mockResolvedValueOnce({
    tags: [research, work],
    renaming: [],
    deleting: [research.id],
    merging: [],
  });
  await render();
  expect(chip("Research").disabled).toBe(true);
  expect(chip("Work").disabled).toBe(false);
  await act(async () =>
    statusListener({
      tags: [research, work],
      renaming: [],
      deleting: [work.id],
      merging: [],
    })
  );
  expect(chip("Work").disabled).toBe(true);
  await act(async () => chip("Work").click());
  expect(mocks.setEntryTag).not.toHaveBeenCalled();
  await act(async () =>
    statusListener({
      tags: [research],
      renaming: [],
      deleting: [],
      merging: [],
    })
  );
  expect(chip("Work")).toBeUndefined();
});

it("redirects merged assignments and reconciles late losing-ID events from current source", async () => {
  await render();
  mocks.readEntryTags.mockResolvedValue([work.id]);
  await act(async () =>
    statusListener({
      tags: [work],
      renaming: [],
      deleting: [],
      merging: [
        {
          sourceId: research.id,
          destinationId: work.id,
          survivorId: work.id,
          destinationName: work.name,
        },
      ],
    })
  );
  expect(chip("Research")).toBeUndefined();
  expect(chip("Work").getAttribute("aria-pressed")).toBe("true");
  await act(async () =>
    statusListener({ tags: [work], renaming: [], deleting: [], merging: [] })
  );
  mocks.readEntryTags.mockResolvedValue([]);
  await act(async () =>
    changeListener({ entryId: "article", tagId: research.id, assigned: true })
  );
  expect(chip("Work").getAttribute("aria-pressed")).toBe("false");
});

async function renderAnnotation(
  entryId = "popup-parent",
  annotationId = "popup-annotation"
) {
  await act(async () =>
    root.render(
      <EntityTags
        key={`${entryId}/${annotationId}`}
        entryId={entryId}
        annotationId={annotationId}
      />
    )
  );
}

it("reads and assigns the explicit inspected Annotation independently of article selection", async () => {
  await renderAnnotation();
  expect(mocks.readAnnotationTags).toHaveBeenCalledWith(
    expect.any(AbortSignal),
    "popup-parent",
    "popup-annotation"
  );
  expect(mocks.readEntryTags).not.toHaveBeenCalled();
  expect(
    container.querySelector('[data-testid="annotation-tags"]')
  ).not.toBeNull();
  await act(async () => chip("Work").click());
  expect(mocks.setAnnotationTag).toHaveBeenCalledWith(
    "popup-parent",
    "popup-annotation",
    "work",
    true
  );
  expect(mocks.setEntryTag).not.toHaveBeenCalled();
  expect(chip("Work").getAttribute("aria-pressed")).toBe("true");
  mocks.setAnnotationTag.mockRejectedValueOnce(new Error("write failed"));
  await act(async () => chip("Work").click());
  expect(chip("Work").getAttribute("aria-pressed")).toBe("true");
});

it("retains committed annotation changes during an initial read and ignores other targets", async () => {
  let finish!: (ids: string[]) => void;
  mocks.readAnnotationTags.mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      })
  );
  await renderAnnotation();
  await act(async () => {
    annotationListener({
      entryId: "popup-parent",
      annotationId: "other",
      tagId: "research",
      assigned: false,
    });
    annotationListener({
      entryId: "popup-parent",
      annotationId: "popup-annotation",
      tagId: "work",
      assigned: true,
    });
    finish(["research"]);
  });
  expect(chip("Research").getAttribute("aria-pressed")).toBe("true");
  expect(chip("Work").getAttribute("aria-pressed")).toBe("true");
});

it("does not display a completed write on a different annotation target", async () => {
  let finish!: (change: AnnotationTagChange) => void;
  mocks.setAnnotationTag.mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      })
  );
  await renderAnnotation();
  await act(async () => chip("Work").click());
  await renderAnnotation("other-parent", "other-annotation");
  await act(async () =>
    finish({
      entryId: "popup-parent",
      annotationId: "popup-annotation",
      tagId: "work",
      assigned: true,
    })
  );
  expect(chip("Work").getAttribute("aria-pressed")).toBe("false");
});

it("does not fall back to Article assignments for a missing annotation", async () => {
  mocks.readAnnotationTags.mockRejectedValueOnce(
    new Error("missing annotation")
  );
  await renderAnnotation();
  expect(container.querySelector('[role="alert"]')).not.toBeNull();
  expect(mocks.readEntryTags).not.toHaveBeenCalled();
  expect(mocks.setEntryTag).not.toHaveBeenCalled();
});

it("never substitutes Article tags for an explicitly empty annotation ID", async () => {
  mocks.readAnnotationTags.mockRejectedValueOnce(
    new Error("invalid annotation")
  );
  await renderAnnotation("parent", "");
  expect(mocks.readAnnotationTags).toHaveBeenCalledWith(
    expect.any(AbortSignal),
    "parent",
    ""
  );
  expect(mocks.readEntryTags).not.toHaveBeenCalled();
  expect(container.querySelector('[role="alert"]')).not.toBeNull();
});

it("finishes creation and assignment to the original annotation after inspection closes", async () => {
  let finish!: (tag: Tag) => void;
  mocks.createTag.mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      })
  );
  await renderAnnotation();
  await search("New");
  await act(async () => chip("Create “New”").click());
  await act(async () => root.unmount());
  await act(async () => finish({ id: "new", name: "New" }));
  expect(mocks.setAnnotationTag).toHaveBeenCalledWith(
    "popup-parent",
    "popup-annotation",
    "new",
    true
  );
  expect(mocks.pushError).not.toHaveBeenCalled();
});
