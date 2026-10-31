import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import MissingShortcut from "./MissingShortcut";
const mocks = vi.hoisted(() => ({
  readOnly: false,
  setPin: vi.fn().mockResolvedValue(undefined),
  pushError: vi.fn(),
}));
vi.mock("../../providers/PinsContext", () => ({ usePins: () => mocks }));
vi.mock("../../hooks/useLibraryReadOnly", () => ({
  useLibraryReadOnly: () => mocks.readOnly,
}));
vi.mock("../../hooks/useErrorSink", () => ({ useErrorSink: () => mocks }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let root: ReturnType<typeof createRoot>;
afterEach(async () => {
  if (root) await act(async () => root.unmount());
  mocks.readOnly = false;
});
it("identifies an unavailable target and offers a focusable unpin action", async () => {
  const container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
  const target = { type: "topic" as const, id: "lost" };
  await act(async () => root.render(<MissingShortcut target={target} />));
  expect(container.textContent).toContain("Target cannot be found");
  const button = container.querySelector("button")!;
  await act(async () => button.focus());
  expect(document.activeElement).toBe(button);
  await act(async () => button.click());
  expect(mocks.setPin).toHaveBeenCalledWith(target, false);
  mocks.readOnly = true;
  await act(async () => root.render(<MissingShortcut target={target} />));
  expect(container.querySelector("button")!.disabled).toBe(true);
});

it("makes unavailable-target actions reachable from the responsive header", async () => {
  const { default: MissingShortcutsButton } =
    await import("./MissingShortcutsButton");
  const container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
  const target = { type: "feed" as const, id: "missing-feed" };
  await act(async () =>
    root.render(<MissingShortcutsButton targets={[target]} />)
  );
  await act(async () => container.querySelector("button")!.click());
  const dialog = document.querySelector('[role="dialog"]')!;
  expect(dialog.textContent).toContain("Target cannot be found");
  const unpin = dialog.querySelector<HTMLButtonElement>(
    'button[aria-label="Remove feed missing-feed from Shortcuts"]'
  )!;
  await act(async () => unpin.focus());
  expect(document.activeElement).toBe(unpin);
  await act(async () => unpin.click());
  expect(mocks.setPin).toHaveBeenCalledWith(target, false);
});
