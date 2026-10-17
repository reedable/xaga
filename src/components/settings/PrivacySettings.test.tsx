import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { SettingsProvider } from "../../providers/SettingsProvider";
import { PrivacySettings } from "./PrivacySettings";

const platform = vi.hoisted(() => ({
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
const toggle = () => container.querySelector<HTMLInputElement>('input[type="checkbox"]')!;
it("loads the saved opt-in and persists disabling it", async () => {
  platform.getLoadRemoteImage.mockResolvedValue(true);
  await act(async () => root.render(<SettingsProvider><PrivacySettings /></SettingsProvider>));
  expect(toggle().checked).toBe(true);
  await act(async () => toggle().click());
  expect(platform.setLoadRemoteImage).toHaveBeenCalledWith(false);
  expect(toggle().checked).toBe(false);
});
it("waits for persistence before showing opt-in as enabled", async () => {
  let finish!: () => void;
  platform.setLoadRemoteImage.mockReturnValue(new Promise<void>((resolve) => { finish = resolve; }));
  await act(async () => root.render(<SettingsProvider><PrivacySettings /></SettingsProvider>));
  await act(async () => toggle().click());
  expect(platform.setLoadRemoteImage).toHaveBeenCalledWith(true);
  expect(toggle().checked).toBe(false);
  expect(toggle().disabled).toBe(true);
  await act(async () => finish());
  expect(toggle().checked).toBe(true);
});
it("keeps the previous choice and reports failed persistence", async () => {
  platform.setLoadRemoteImage.mockRejectedValue(new Error("write failed"));
  await act(async () => root.render(<SettingsProvider><PrivacySettings /></SettingsProvider>));
  await act(async () => toggle().click());
  expect(toggle().checked).toBe(false);
  expect(sink.pushError).toHaveBeenCalledWith(expect.any(Error));
  expect(toggle().disabled).toBe(false);
});
it("disables the switch when the saved preference cannot be read", async () => {
  platform.getLoadRemoteImage.mockRejectedValue(new Error("read failed"));
  await act(async () => root.render(<SettingsProvider><PrivacySettings /></SettingsProvider>));
  expect(toggle().disabled).toBe(true);
  expect(container.textContent).toContain("Could not load");
  expect(platform.setLoadRemoteImage).not.toHaveBeenCalled();
});
