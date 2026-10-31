import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import EditSiteDialog from "./EditSiteDialog";
const mocks = vi.hoisted(() => ({
  pins: [] as import("../../pins").PinTarget[],
  setPin: vi.fn().mockResolvedValue(undefined),
  readOnly: false,
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
  document.body.innerHTML = "";
  mocks.readOnly = false;
  mocks.pins = [];
  vi.clearAllMocks();
});
const site = {
  id: "example.com",
  hostname: "example.com",
  title: "example.com",
  entryCount: 0,
  unreadCount: 0,
};
it("restores persisted membership on reopen, saves the typed Site reference, and leaves identity read-only", async () => {
  root = createRoot(document.createElement("div"));
  const onClose = vi.fn();
  const render = (open: boolean) => (
    <EditSiteDialog site={open ? site : null} onClose={onClose} />
  );
  mocks.pins = [{ type: "site", hostname: site.hostname }];
  await act(async () => root.render(render(true)));
  const checkbox = () =>
    document.querySelector<HTMLInputElement>('input[type="checkbox"]')!;
  expect(checkbox().checked).toBe(true);
  const hostname = document.querySelector<HTMLInputElement>(
    'input[value="example.com"]'
  )!;
  expect(hostname.readOnly).toBe(true);
  await act(async () => checkbox().click());
  expect(checkbox().checked).toBe(false);
  await act(async () => root.render(render(false)));
  await act(async () => root.render(render(true)));
  expect(checkbox().checked).toBe(true);
  await act(async () => checkbox().click());
  await act(async () =>
    document
      .querySelector("form")!
      .dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }))
  );
  expect(mocks.setPin).toHaveBeenCalledWith(
    { type: "site", hostname: "example.com" },
    false
  );
  expect(onClose).toHaveBeenCalled();
});
it("disables pin changes and submission in read-only mode", async () => {
  root = createRoot(document.createElement("div"));
  mocks.readOnly = true;
  await act(async () =>
    root.render(<EditSiteDialog site={site} onClose={vi.fn()} />)
  );
  expect(
    document.querySelector<HTMLInputElement>('input[type="checkbox"]')!.disabled
  ).toBe(true);
  await act(async () =>
    document
      .querySelector("form")!
      .dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }))
  );
  expect(mocks.setPin).not.toHaveBeenCalled();
});
