import { act, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { MemoryRouter } from "react-router-dom";
import { afterEach, expect, it, vi } from "vitest";
import useRouter from "./useRouter";
import type { NavigationState } from "../types";

vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let root: ReturnType<typeof createRoot> | undefined;
let container: HTMLDivElement;
let router: ReturnType<typeof useRouter>;
function Host() {
  const value = useRouter();
  useEffect(() => {
    router = value;
  }, [value]);
  return null;
}
async function render(state: NavigationState) {
  container = document.createElement("div");
  root = createRoot(container);
  await act(async () =>
    root!.render(
      <MemoryRouter initialEntries={[{ pathname: "/", state }]}>
        <Host />
      </MemoryRouter>
    )
  );
}
afterEach(async () => {
  await act(async () => root?.unmount());
});

it("keeps the resolved Article URL when selecting another Annotation in that Article", async () => {
  await render({
    entryId: "article",
    annotationId: "first",
    url: "https://example.org/article",
    mimeType: "text/html",
  });
  await act(async () =>
    router.selectAnnotation({ id: "second", reference: "article" })
  );
  expect(router.state).toMatchObject({
    entryId: "article",
    annotationId: "second",
    url: "https://example.org/article",
    mimeType: "text/html",
  });
});

it("clears the previous Article URL when selecting an Annotation in a different Article", async () => {
  await render({
    entryId: "article",
    url: "https://example.org/article",
    mimeType: "text/html",
  });
  await act(async () =>
    router.selectAnnotation({ id: "annotation", reference: "other" })
  );
  expect(router.state.entryId).toBe("other");
  expect(router.state.annotationId).toBe("annotation");
  expect(router.state.url).toBeUndefined();
  expect(router.state.mimeType).toBeUndefined();
});

it("uses the URL already carried by an Annotation row when changing Articles", async () => {
  await render({ entryId: "article", url: "https://example.org/article" });
  await act(async () =>
    router.selectAnnotation({
      id: "annotation",
      reference: "other",
      url: "https://example.org/other",
    })
  );
  expect(router.state).toMatchObject({
    entryId: "other",
    annotationId: "annotation",
    url: "https://example.org/other",
  });
});
