import { act, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { beforeEach, afterEach, expect, it, vi } from "vitest";
import PinsProvider from "./PinsProvider";
import { usePins } from "./PinsContext";
import type { PinChange, PinTarget } from "../pins";

const mocks = vi.hoisted(() => ({
  listPins: vi.fn(),
  setPin: vi.fn(),
  onPinChanged: vi.fn(),
  pushError: vi.fn(),
}));
vi.mock("../platform/Platform", () => ({ usePlatform: () => mocks }));
vi.mock("../hooks/useErrorSink", () => ({ useErrorSink: () => mocks }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let current: ReturnType<typeof usePins>;
let changed: (change: PinChange) => void;
let root: ReturnType<typeof createRoot>;
function Probe() {
  const value = usePins();
  useEffect(() => {
    current = value;
  });
  return null;
}
const target: PinTarget = { type: "entry", id: "missing" };
beforeEach(() => {
  vi.clearAllMocks();
  mocks.listPins.mockResolvedValue([target]);
  mocks.setPin.mockResolvedValue(undefined);
  mocks.onPinChanged.mockImplementation(async (callback) => {
    changed = callback;
    return () => {};
  });
  root = createRoot(document.createElement("div"));
});
afterEach(async () => {
  await act(async () => root.unmount());
});
const render = () =>
  act(async () =>
    root.render(
      <PinsProvider>
        <Probe />
      </PinsProvider>
    )
  );
it("retains unresolved targets and refuses to optimistically display failed writes", async () => {
  await render();
  expect(current.pins).toEqual([target]);
  const error = new Error("read-only");
  mocks.setPin.mockRejectedValueOnce(error);
  await act(async () => {
    await expect(current.setPin(target, false)).rejects.toBe(error);
  });
  expect(current.pins).toEqual([target]);
});
it("rereads the authority for delayed events and ignores stale snapshots", async () => {
  let initial!: (targets: PinTarget[]) => void;
  mocks.listPins.mockImplementationOnce(
    () =>
      new Promise<PinTarget[]>((resolve) => {
        initial = resolve;
      })
  );
  await render();
  mocks.listPins.mockResolvedValue([]);
  await act(async () => changed({ target, pinned: false }));
  await act(async () => initial([target]));
  expect(current.pins).toEqual([]);
  // An older notification cannot re-pin something already removed in storage.
  await act(async () => changed({ target, pinned: true }));
  expect(current.pins).toEqual([]);
});
