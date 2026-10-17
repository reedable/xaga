import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { SettingsProvider } from "../../providers/SettingsProvider";
import { GeneralSettings } from "./GeneralSettings";

const MIB = 1024 * 1024;
const platform = vi.hoisted(() => ({
  onWindowCloseRequested: vi.fn().mockResolvedValue(() => {}),
  getLibrary: vi.fn().mockResolvedValue("library"),
  getRefreshOnStartup: vi.fn().mockResolvedValue(false),
  getInlinePdfReader: vi.fn().mockResolvedValue(false),
  onOpenLibrary: vi.fn().mockResolvedValue(() => {}),
  onRefreshOnStartupChanged: vi.fn().mockResolvedValue(() => {}),
  onInlinePdfReaderChanged: vi.fn().mockResolvedValue(() => {}),
  getImportMaxBytes: vi.fn(),
  setImportMaxBytes: vi.fn(),
}));
const sink = vi.hoisted(() => ({ pushError: vi.fn() }));
vi.mock("../../platform/Platform", () => ({ usePlatform: () => platform }));
vi.mock("../../hooks/useErrorSink", () => ({ useErrorSink: () => sink }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let root: ReturnType<typeof createRoot>;
let container: HTMLDivElement;
let storedBytes: number;
beforeEach(async () => {
  vi.clearAllMocks();
  storedBytes = 64 * MIB;
  platform.getImportMaxBytes
    .mockReset()
    .mockImplementation(async () => storedBytes);
  platform.setImportMaxBytes
    .mockReset()
    .mockImplementation(async (bytes: number) => {
      storedBytes = bytes;
    });
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
  await act(async () =>
    root.render(
      <SettingsProvider>
        <GeneralSettings />
      </SettingsProvider>
    )
  );
});
afterEach(async () => {
  await act(async () => root.unmount());
  container.remove();
});
const input = () =>
  container.querySelector<HTMLInputElement>('input[type="number"]')!;
async function blur() {
  await act(async () =>
    input().dispatchEvent(new FocusEvent("focusout", { bubbles: true }))
  );
}
async function enter() {
  await act(async () =>
    input().dispatchEvent(
      new KeyboardEvent("keydown", { key: "Enter", bubbles: true })
    )
  );
}
async function edit(value: string) {
  await act(async () => {
    Object.getOwnPropertyDescriptor(
      HTMLInputElement.prototype,
      "value"
    )!.set!.call(input(), value);
    input().dispatchEvent(new Event("input", { bubbles: true }));
  });
}

it("loads bytes as MiB and saves the converted value", async () => {
  expect(input().value).toBe("64");
  expect(container.querySelector('button[type="submit"]')).toBeNull();
  await edit("128");
  await blur();
  expect(platform.setImportMaxBytes).toHaveBeenCalledWith(128 * MIB);
  expect(storedBytes).toBe(128 * MIB);

  await act(async () =>
    root.render(
      <SettingsProvider>
        <GeneralSettings key="reopened" />
      </SettingsProvider>
    )
  );
  expect(input().value).toBe("128");
});

it.each(["", "0", "-1", "0.5", "1.5", "0.0000001", "8589934592"])(
  "rejects invalid MiB value %s before invoking native code",
  async (value) => {
    await edit(value);

    expect(input().getAttribute("aria-invalid")).toBe("true");
    await blur();
    expect(platform.setImportMaxBytes).not.toHaveBeenCalled();
  }
);

it("saves on Enter and does not repeat an unchanged save on blur", async () => {
  await edit("128");
  expect(platform.setImportMaxBytes).not.toHaveBeenCalled();
  await enter();
  expect(platform.setImportMaxBytes).toHaveBeenCalledWith(128 * MIB);
  await blur();
  expect(platform.setImportMaxBytes).toHaveBeenCalledTimes(1);
});

it("waits for persistence and prevents overlapping saves", async () => {
  let finish!: () => void;
  platform.setImportMaxBytes.mockReturnValueOnce(
    new Promise<void>((resolve) => {
      finish = resolve;
    })
  );
  await edit("128");
  await blur();
  expect(input().disabled).toBe(true);

  await enter();
  expect(platform.setImportMaxBytes).toHaveBeenCalledTimes(1);
  await act(async () => finish());
  expect(input().disabled).toBe(false);
});

it.each(["settings:busy", "settings:stale-view"])(
  "reloads the actual limit after %s and allows retry",
  async (code) => {
    const error = { code, message: "Please retry", detail: "" };
    platform.setImportMaxBytes.mockImplementationOnce(async () => {
      storedBytes = 96 * MIB;
      throw error;
    });
    await edit("128");
    await blur();
    expect(sink.pushError).toHaveBeenCalledWith(error);
    expect(input().value).toBe("96");

    await edit("128");
    await blur();
    expect(storedBytes).toBe(128 * MIB);
  }
);

it("saves the latest edit when closing without blur or Enter", async () => {
  await edit("128");
  expect(platform.setImportMaxBytes).not.toHaveBeenCalled();
  const close = platform.onWindowCloseRequested.mock.calls[0][0];
  let allowed: boolean | undefined;
  await act(async () => {
    allowed = await close();
  });
  expect(allowed).toBe(true);
  expect(storedBytes).toBe(128 * MIB);
  expect(platform.onWindowCloseRequested).toHaveBeenCalledTimes(1);
});

it("waits for an in-flight save before allowing close, without another write", async () => {
  let finish!: () => void;
  platform.setImportMaxBytes.mockReturnValueOnce(
    new Promise<void>((resolve) => {
      finish = resolve;
    })
  );
  await edit("128");
  await blur();
  let allowed: boolean | undefined;
  const close = platform.onWindowCloseRequested.mock.calls[0][0];
  let closing!: Promise<void>;
  await act(async () => {
    closing = close().then((value: boolean) => {
      allowed = value;
    });
  });
  expect(allowed).toBeUndefined();
  expect(platform.setImportMaxBytes).toHaveBeenCalledTimes(1);
  await act(async () => {
    finish();
    await closing;
  });
  expect(allowed).toBe(true);
});

it("keeps the window open when saving on close fails", async () => {
  const error = { code: "settings:busy", message: "Please retry", detail: "" };
  platform.setImportMaxBytes.mockRejectedValueOnce(error);
  await edit("128");
  const close = platform.onWindowCloseRequested.mock.calls[0][0];
  let allowed: boolean | undefined;
  await act(async () => {
    allowed = await close();
  });
  expect(allowed).toBe(false);
  expect(sink.pushError).toHaveBeenCalledWith(error);
  expect(input().value).toBe("64");
});

it("keeps invalid edits visible instead of closing and silently discarding them", async () => {
  await edit("1.5");
  const close = platform.onWindowCloseRequested.mock.calls[0][0];
  let allowed: boolean | undefined;
  await act(async () => {
    allowed = await close();
  });
  expect(allowed).toBe(false);
  expect(platform.setImportMaxBytes).not.toHaveBeenCalled();
  expect(input().getAttribute("aria-invalid")).toBe("true");
});
