import { afterEach, expect, it, vi } from "vitest";

afterEach(() => {
  vi.unstubAllEnvs();
  vi.resetModules();
});

it.each([
  [true, "capture", true],
  [true, "development", false],
  [false, "capture", false],
  [false, "production", false],
])(
  "selects fixtures only in explicit development capture mode (%s, %s)",
  async (dev, mode, expected) => {
    vi.stubEnv("DEV", dev);
    vi.stubEnv("MODE", mode);
    vi.resetModules();
    expect((await import("./captureMode")).isCaptureMode).toBe(expected);
  }
);

it("keeps capture view state in memory and discards it on a fresh startup", async () => {
  vi.stubEnv("DEV", true);
  vi.stubEnv("MODE", "capture");
  vi.resetModules();
  const storage = await import("../storage/viewState");
  await storage.setViewState("sidebar.expandedList", "FEEDS");
  expect(await storage.getViewState("sidebar.expandedList")).toBe("FEEDS");
  vi.resetModules();
  expect(
    await (
      await import("../storage/viewState")
    ).getViewState("sidebar.expandedList")
  ).toBeUndefined();
});
