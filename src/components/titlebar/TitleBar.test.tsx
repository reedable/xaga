import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { TitleBar } from "./TitleBar";

const native = vi.hoisted(() => ({
  startDragging: vi.fn().mockResolvedValue(undefined),
  toggleMaximize: vi.fn().mockResolvedValue(undefined),
}));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => native }));
vi.mock("../../platform/captureMode", () => ({ isCaptureMode: false }));

vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);

let cleanup: (() => void) | undefined;
afterEach(() => {
  cleanup?.();
  vi.clearAllMocks();
});

function mount() {
  const container = document.createElement("div");
  document.body.append(container);
  const root = createRoot(container);
  act(() => root.render(<TitleBar />));
  cleanup = () => {
    act(() => root.unmount());
    container.remove();
  };
  return container.firstElementChild as HTMLElement;
}

it("requests native dragging for primary mouse down on the titlebar", () => {
  const bar = mount();
  act(() =>
    bar.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, button: 0 }))
  );
  expect(native.startDragging).toHaveBeenCalledTimes(1);
  act(() =>
    bar.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, button: 2 }))
  );
  expect(native.startDragging).toHaveBeenCalledTimes(1);
});

it("requests native maximize toggling on titlebar double click", () => {
  const bar = mount();
  act(() => bar.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
  expect(native.toggleMaximize).toHaveBeenCalledTimes(1);
});

it("leaves interactive descendants to handle their own pointer input", () => {
  const bar = mount();
  for (const tag of ["button", "input", "a", "textarea"]) {
    const control = document.createElement(tag);
    bar.append(control);
    act(() => {
      control.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
      control.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
    });
  }
  expect(native.startDragging).not.toHaveBeenCalled();
  expect(native.toggleMaximize).not.toHaveBeenCalled();
});
