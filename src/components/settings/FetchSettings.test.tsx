import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { SettingsProvider } from "../../providers/SettingsProvider";
import { FetchSettingsProvider } from "./FetchSettingsProvider";
import { SettingsSaveProvider } from "./SettingsSaveProvider";
import { GeneralSettings } from "./GeneralSettings";
import { SecuritySettings } from "./SecuritySettings";
import { FetchSwitch } from "./FetchSettingControls";
import type {
  FetchSettings,
  FetchSettingChange,
} from "../../platform/fetchSettings";

const platform = vi.hoisted(() => ({
  getLibrary: vi.fn().mockResolvedValue("library"),
  getRefreshOnStartup: vi.fn().mockResolvedValue(false),
  getInlinePdfReader: vi.fn().mockResolvedValue(false),
  onOpenLibrary: vi.fn().mockResolvedValue(() => {}),
  onRefreshOnStartupChanged: vi.fn().mockResolvedValue(() => {}),
  onInlinePdfReaderChanged: vi.fn().mockResolvedValue(() => {}),
  onWindowCloseRequested: vi.fn().mockResolvedValue(() => {}),
  getImportMaxBytes: vi.fn().mockResolvedValue(64 * 1024 * 1024),
  getFetchSettings: vi.fn(),
  setFetchSetting: vi.fn(),
}));
const sink = vi.hoisted(() => ({ pushError: vi.fn() }));
vi.mock("../../platform/Platform", () => ({ usePlatform: () => platform }));
vi.mock("../../hooks/useErrorSink", () => ({ useErrorSink: () => sink }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let root: ReturnType<typeof createRoot>;
let container: HTMLDivElement;
let stored: FetchSettings;
beforeEach(async () => {
  vi.clearAllMocks();
  stored = {
    refererAllowed: false,
    gzipAllowed: false,
    httpAllowed: false,
    connectNonstandardPortsAllowed: false,
    maxRedirects: "3",
    loaderTimeoutSecs: "10",
    feedTimeoutSecs: "10",
    feedMaxBodyBytes: "10485760",
  };
  platform.getFetchSettings
    .mockReset()
    .mockImplementation(async () => ({ ...stored }));
  platform.setFetchSetting
    .mockReset()
    .mockImplementation(async ({ setting, value }: FetchSettingChange) => {
      stored = { ...stored, [setting]: value };
    });
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
  await render();
});
async function render() {
  await act(async () =>
    root.render(
      <SettingsProvider>
        <SettingsSaveProvider>
          <FetchSettingsProvider>
            <GeneralSettings />
            <SecuritySettings />
            <FetchSwitch
              setting="refererAllowed"
              label="Send referring URLs"
              description="Feed redirects"
            />
            <FetchSwitch
              setting="gzipAllowed"
              label="Allow compression"
              description="Feed transfers"
            />
          </FetchSettingsProvider>
        </SettingsSaveProvider>
      </SettingsProvider>
    )
  );
}
afterEach(async () => {
  await act(async () => root.unmount());
  container.remove();
});
function input(setting: string) {
  return container.querySelector<HTMLInputElement>(`input[name="${setting}"]`)!;
}

async function edit(label: string, value: string) {
  await act(async () => {
    Object.getOwnPropertyDescriptor(
      HTMLInputElement.prototype,
      "value"
    )!.set!.call(input(label), value);
    input(label).dispatchEvent(new Event("input", { bubbles: true }));
  });
}
async function blur(label: string) {
  await act(async () => {
    input(label).dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
  });
}
async function close() {
  let allowed = false;
  await act(async () => {
    allowed = await platform.onWindowCloseRequested.mock.calls[0][0]();
  });
  return allowed;
}

it.each([
  ["loaderTimeoutSecs", "loaderTimeoutSecs"],
  ["feedTimeoutSecs", "feedTimeoutSecs"],
] as const)(
  "preserves the full u64 range for %s through reads and writes",
  async (label, setting) => {
    await edit(label, "18446744073709551615");
    await blur(label);
    expect(platform.setFetchSetting).toHaveBeenLastCalledWith({
      setting,
      value: "18446744073709551615",
    });
    expect(input(label).value).toBe("18446744073709551615");
    await edit(label, "0");
    await blur(label);
    expect(stored[setting]).toBe("0");
    for (const value of ["-1", "1.5", "1e2", "18446744073709551616", ""]) {
      const calls = platform.setFetchSetting.mock.calls.length;
      await edit(label, value);
      await blur(label);
      expect(input(label).getAttribute("aria-invalid")).toBe("true");
      expect(platform.setFetchSetting).toHaveBeenCalledTimes(calls);
    }
  }
);
it("edits feed limits in MiB and preserves exact byte values", async () => {
  const label = "feedMaxBodyBytes";
  expect(input(label).value).toBe("10");
  for (const [mib, bytes] of [
    ["20", "20971520"],
    ["1.5", "1572864"],
    ["0", "0"],
    ["0.00000095367431640625", "1"],
    ["17592186044415.99999904632568359375", "18446744073709551615"],
  ]) {
    await edit(label, mib);
    await blur(label);
    expect(stored.feedMaxBodyBytes).toBe(bytes);
    expect(input(label).value).toBe(mib);
  }
  for (const value of ["-1", "1e2", "0.0000001", "17592186044416", ""]) {
    const calls = platform.setFetchSetting.mock.calls.length;
    await edit(label, value);
    await blur(label);
    expect(input(label).getAttribute("aria-invalid")).toBe("true");
    expect(platform.setFetchSetting).toHaveBeenCalledTimes(calls);
  }
});

it("enforces the existing redirect range including zero", async () => {
  await edit("maxRedirects", "11");
  expect(await close()).toBe(false);
  expect(platform.setFetchSetting).not.toHaveBeenCalled();
  await edit("maxRedirects", "0");
  expect(await close()).toBe(true);
  expect(stored.maxRedirects).toBe("0");
});
it("retains a failed numeric draft and saves it on retry", async () => {
  platform.setFetchSetting.mockRejectedValueOnce(new Error("write failed"));
  await edit("feedTimeoutSecs", "9007199254740993");
  expect(await close()).toBe(false);
  expect(stored.feedTimeoutSecs).toBe("10");
  expect(input("feedTimeoutSecs").value).toBe("9007199254740993");
  expect(sink.pushError).toHaveBeenCalledOnce();
  expect(await close()).toBe(true);
  expect(stored.feedTimeoutSecs).toBe("9007199254740993");
});
it("uses one close handler and waits for every numeric save, including in-flight work", async () => {
  let finish!: () => void;
  platform.setFetchSetting.mockImplementationOnce(
    () =>
      new Promise<void>((resolve) => {
        finish = resolve;
      })
  );
  await edit("loaderTimeoutSecs", "25");
  await blur("loaderTimeoutSecs");
  await edit("feedTimeoutSecs", "35");
  let allowed: boolean | undefined;
  let closing!: Promise<void>;
  await act(async () => {
    const requestClose = platform.onWindowCloseRequested.mock.calls[0][0];
    closing = requestClose().then((value: boolean) => {
      allowed = value;
    });
  });
  expect(allowed).toBeUndefined();
  expect(platform.onWindowCloseRequested).toHaveBeenCalledOnce();
  await act(async () => {
    finish();
    await closing;
  });
  expect(allowed).toBe(true);
  expect(stored.feedTimeoutSecs).toBe("35");
  expect(platform.setFetchSetting).toHaveBeenCalledTimes(2);
});
it.each([
  "httpAllowed",
  "connectNonstandardPortsAllowed",
  "refererAllowed",
  "gzipAllowed",
])("keeps %s unchanged until persistence succeeds", async (label) => {
  let finish!: () => void;
  platform.setFetchSetting.mockImplementationOnce(
    (change: FetchSettingChange) =>
      new Promise<void>((resolve) => {
        finish = () => {
          stored = { ...stored, [change.setting]: change.value };
          resolve();
        };
      })
  );
  expect(input(label).checked).toBe(false);
  await act(async () => input(label).click());
  expect(input(label).checked).toBe(false);
  expect(input(label).disabled).toBe(true);
  await act(async () => finish());
  expect(input(label).checked).toBe(true);
  platform.setFetchSetting.mockRejectedValueOnce(new Error("write failed"));
  await act(async () => input(label).click());
  expect(input(label).checked).toBe(true);
  expect(input(label).disabled).toBe(false);
});
