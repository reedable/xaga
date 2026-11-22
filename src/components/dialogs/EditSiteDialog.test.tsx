import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import EditSiteDialog from "./EditSiteDialog";
const mocks = vi.hoisted(() => ({
  tree: [] as import("../../pins").PinNode[],
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
  mocks.tree = [];
  vi.clearAllMocks();
});
const site = {
  id: "example.com",
  __kind: "site" as const,
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
  mocks.tree = [{ type: "site", hostname: site.id }];
  await act(async () => root.render(render(true)));
  const destination = () =>
    document.querySelector<HTMLSelectElement>(
      '[data-testid="shortcut-location"]'
    )!;
  const choose = (value: string) =>
    act(() => {
      destination().value = value;
      destination().dispatchEvent(new Event("change", { bubbles: true }));
    });
  expect(destination().value).toBe("root");
  const hostname = document.querySelector<HTMLInputElement>(
    'input[value="example.com"]'
  )!;
  expect(hostname.readOnly).toBe(true);
  await choose("none");
  expect(destination().value).toBe("none");
  await act(async () => root.render(render(false)));
  await act(async () => root.render(render(true)));
  expect(destination().value).toBe("root");
  await choose("none");
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
    document.querySelector<HTMLSelectElement>(
      '[data-testid="shortcut-location"]'
    )!.disabled
  ).toBe(true);
  await act(async () =>
    document
      .querySelector("form")!
      .dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }))
  );
  expect(mocks.setPin).not.toHaveBeenCalled();
});
