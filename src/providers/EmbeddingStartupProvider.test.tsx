import { act, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { EmbeddingStartupState } from "../platform/Platform";
import { useEmbeddingStartupState } from "../hooks/useEmbeddingStartupState";
import { EmbeddingStartupProvider } from "./EmbeddingStartupProvider";

const platform = vi.hoisted(() => ({ embeddingStartupState: vi.fn() }));
vi.mock("../platform/Platform", () => ({ usePlatform: () => platform }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);

let observed: EmbeddingStartupState[];
let root: ReturnType<typeof createRoot>;

function Capture() {
  const state = useEmbeddingStartupState();
  useEffect(() => {
    observed.push(state);
  }, [state]);
  return null;
}

beforeEach(() => {
  vi.clearAllMocks();
  observed = [];
  root = createRoot(document.createElement("div"));
});

afterEach(async () => {
  await act(async () => root.unmount());
});

async function renderProvider() {
  await act(async () => {
    root.render(
      <EmbeddingStartupProvider>
        <Capture />
      </EmbeddingStartupProvider>
    );
  });
}

it("reports loading while initialization remains in progress", async () => {
  platform.embeddingStartupState.mockReturnValue(new Promise(() => {}));
  await renderProvider();
  expect(observed).toEqual(["loading"]);
});

it("reaches the available terminal state", async () => {
  platform.embeddingStartupState.mockResolvedValue("available");
  await renderProvider();
  expect(observed).toEqual(["loading", "available"]);
});

it("reaches the unavailable terminal state", async () => {
  platform.embeddingStartupState.mockResolvedValue("unavailable");
  await renderProvider();
  expect(observed).toEqual(["loading", "unavailable"]);
});

it("fails open to unavailable when the startup-state command fails", async () => {
  platform.embeddingStartupState.mockRejectedValue(new Error("IPC failed"));
  await renderProvider();
  expect(observed).toEqual(["loading", "unavailable"]);
});
