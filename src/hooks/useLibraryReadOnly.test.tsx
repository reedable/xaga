import { act, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { useLibraryReadOnly } from "./useLibraryReadOnly";
const mocks = vi.hoisted(() => ({
  platform: { isLibraryReadOnly: vi.fn(), onLibraryModeChanged: vi.fn() },
}));
vi.mock("../platform/Platform", () => ({ usePlatform: () => mocks.platform }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let root: ReturnType<typeof createRoot>;
afterEach(async () => {
  await act(() => root.unmount());
  vi.resetAllMocks();
});
it("disables writes while mode is delayed and ignores an older command after a mode event", async () => {
  let resolve!: (value: boolean) => void;
  let announce!: (value: boolean) => void;
  mocks.platform.isLibraryReadOnly.mockReturnValue(
    new Promise<boolean>((done) => {
      resolve = done;
    })
  );
  mocks.platform.onLibraryModeChanged.mockImplementation((callback) => {
    announce = callback;
    return Promise.resolve(() => {});
  });
  let readOnly = false;
  function Host() {
    const value = useLibraryReadOnly();
    useEffect(() => {
      readOnly = value;
    }, [value]);
    return null;
  }
  root = createRoot(document.createElement("div"));
  await act(() => root.render(<Host />));
  expect(readOnly).toBe(true);
  await act(() => announce(true));
  await act(() => resolve(false));
  expect(readOnly).toBe(true);
  await act(() => announce(false));
  expect(readOnly).toBe(false);
});
