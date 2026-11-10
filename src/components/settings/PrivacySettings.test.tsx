import { FetchSettingsProvider } from "./FetchSettingsProvider";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { SettingsProvider } from "../../providers/SettingsProvider";
import { PrivacySettings } from "./PrivacySettings";

const platform = vi.hoisted(() => ({
  getFetchSettings: vi.fn().mockResolvedValue({
    refererAllowed: false,
    gzipAllowed: false,
    httpAllowed: false,
    connectNonstandardPortsAllowed: false,
    maxRedirects: "3",
    loaderTimeoutSecs: "10",
    feedTimeoutSecs: "10",
    feedMaxBodyBytes: "10485760",
  }),
  setFetchSetting: vi.fn().mockResolvedValue(undefined),
  getLibrary: vi.fn().mockResolvedValue("library"),
  getRefreshOnStartup: vi.fn().mockResolvedValue(false),
  getInlinePdfReader: vi.fn().mockResolvedValue(false),
  onOpenLibrary: vi.fn().mockResolvedValue(() => {}),
  onRefreshOnStartupChanged: vi.fn().mockResolvedValue(() => {}),
  onInlinePdfReaderChanged: vi.fn().mockResolvedValue(() => {}),
  setAllowedDomains: vi.fn().mockResolvedValue(undefined),
  getAllowedDomains: vi.fn().mockResolvedValue([]),
  getLoadRemoteImage: vi.fn(),
  setLoadRemoteImage: vi.fn(),
}));
vi.mock("../../platform/Platform", () => ({ usePlatform: () => platform }));
const sink = vi.hoisted(() => ({ pushError: vi.fn() }));
vi.mock("../../hooks/useErrorSink", () => ({ useErrorSink: () => sink }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let container: HTMLDivElement;
let root: ReturnType<typeof createRoot>;
beforeEach(() => {
  vi.clearAllMocks();
  platform.getLoadRemoteImage.mockResolvedValue(false);
  platform.setLoadRemoteImage.mockResolvedValue(undefined);
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
});
afterEach(async () => {
  await act(async () => root.unmount());
  container.remove();
});
const toggle = () =>
  Array.from(
    container.querySelectorAll<HTMLInputElement>('input[type="checkbox"]')
  )[2]!;
it("loads the saved opt-in and persists disabling it", async () => {
  platform.getLoadRemoteImage.mockResolvedValue(true);
  await act(async () =>
    root.render(
      <SettingsProvider>
        <FetchSettingsProvider>
          <PrivacySettings />
        </FetchSettingsProvider>
      </SettingsProvider>
    )
  );
  expect(toggle().checked).toBe(true);
  await act(async () => toggle().click());
  expect(platform.setLoadRemoteImage).toHaveBeenCalledWith(false);
  expect(toggle().checked).toBe(false);
});
it("waits for persistence before showing opt-in as enabled", async () => {
  let finish!: () => void;
  platform.setLoadRemoteImage.mockReturnValue(
    new Promise<void>((resolve) => {
      finish = resolve;
    })
  );
  await act(async () =>
    root.render(
      <SettingsProvider>
        <FetchSettingsProvider>
          <PrivacySettings />
        </FetchSettingsProvider>
      </SettingsProvider>
    )
  );
  await act(async () => toggle().click());
  expect(platform.setLoadRemoteImage).toHaveBeenCalledWith(true);
  expect(toggle().checked).toBe(false);
  expect(toggle().disabled).toBe(true);
  await act(async () => finish());
  expect(toggle().checked).toBe(true);
});
it("keeps the previous choice and reports failed persistence", async () => {
  platform.setLoadRemoteImage.mockRejectedValue(new Error("write failed"));
  await act(async () =>
    root.render(
      <SettingsProvider>
        <FetchSettingsProvider>
          <PrivacySettings />
        </FetchSettingsProvider>
      </SettingsProvider>
    )
  );
  await act(async () => toggle().click());
  expect(toggle().checked).toBe(false);
  expect(sink.pushError).toHaveBeenCalledWith(expect.any(Error));
  expect(toggle().disabled).toBe(false);
});
it("disables the switch when the saved preference cannot be read", async () => {
  platform.getLoadRemoteImage.mockRejectedValue(new Error("read failed"));
  await act(async () =>
    root.render(
      <SettingsProvider>
        <FetchSettingsProvider>
          <PrivacySettings />
        </FetchSettingsProvider>
      </SettingsProvider>
    )
  );
  expect(toggle().disabled).toBe(true);
  expect(container.querySelector('[role="alert"]')).not.toBeNull();
  expect(platform.setLoadRemoteImage).not.toHaveBeenCalled();
});
