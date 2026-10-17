import { act, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { SettingsProvider } from "./SettingsProvider";
import { useSettings } from "../hooks/useSettings";

const platform = vi.hoisted(() => ({
  getLibrary: vi.fn().mockResolvedValue("library"),
  getRefreshOnStartup: vi.fn(),
  getInlinePdfReader: vi.fn(),
  setRefreshOnStartup: vi.fn(),
  setInlinePdfReader: vi.fn(),
  onOpenLibrary: vi.fn().mockResolvedValue(() => {}),
  onRefreshOnStartupChanged: vi.fn().mockResolvedValue(() => {}),
  onInlinePdfReaderChanged: vi.fn().mockResolvedValue(() => {}),
}));
const sink = vi.hoisted(() => ({ pushError: vi.fn() }));
vi.mock("../platform/Platform", () => ({ usePlatform: () => platform }));
vi.mock("../hooks/useErrorSink", () => ({ useErrorSink: () => sink }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let settings: ReturnType<typeof useSettings>;
function Capture() {
  const value = useSettings();
  useEffect(() => {
    settings = value;
  });
  return null;
}
let root: ReturnType<typeof createRoot>;
beforeEach(async () => {
  vi.clearAllMocks();
  platform.getRefreshOnStartup.mockResolvedValue(false);
  platform.getInlinePdfReader.mockResolvedValue(false);
  platform.setRefreshOnStartup.mockResolvedValue(undefined);
  platform.setInlinePdfReader.mockResolvedValue(undefined);
  root = createRoot(document.createElement("div"));
  await act(async () =>
    root.render(
      <SettingsProvider>
        <Capture />
      </SettingsProvider>
    )
  );
});
afterEach(async () => {
  await act(async () => root.unmount());
});

it("does not show a pending or refused write as saved", async () => {
  let reject!: (e: unknown) => void;
  platform.setRefreshOnStartup.mockReturnValue(
    new Promise((_, fail) => {
      reject = fail;
    })
  );
  await act(async () => settings.setRefreshOnStartup(true));
  expect(settings.refreshOnStartup).toBe(false);
  const busy = { code: "settings:busy", message: "Try later", detail: "" };
  await act(async () => reject(busy));
  expect(settings.refreshOnStartup).toBe(false);
  expect(sink.pushError).toHaveBeenCalledWith(busy);
  expect(platform.getRefreshOnStartup).toHaveBeenCalledTimes(2);
});

it("reloads all settings after stale-view refusal and permits retry", async () => {
  const stale = { code: "settings:stale-view", message: "Retry", detail: "" };
  platform.setRefreshOnStartup.mockRejectedValueOnce(stale);
  platform.getInlinePdfReader.mockResolvedValue(true);
  await act(async () => settings.setRefreshOnStartup(true));
  expect(settings.refreshOnStartup).toBe(false);
  expect(settings.inlinePdfReader).toBe(true);
  expect(settings.refreshVersion).toBe(1);
  expect(sink.pushError).toHaveBeenCalledWith(stale);
  await act(async () => settings.setRefreshOnStartup(true));
  expect(settings.refreshOnStartup).toBe(true);
});
