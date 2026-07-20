import { beforeEach, describe, expect, it, vi } from "vitest";

// Pins the openUrl scheme allowlist: only web/mail schemes reach the system
// opener; file:/javascript:/data: and custom schemes must be refused before
// the plugin is called.
const { openUrlMock } = vi.hoisted(() => ({ openUrlMock: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: openUrlMock }));
// TauriPlatform registers a module-load event listener; no-op it under test.
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

import { TauriPlatform } from "./TauriPlatform";

describe("TauriPlatform.openUrl scheme allowlist", () => {
  const platform = new TauriPlatform();

  beforeEach(() => openUrlMock.mockClear());

  it.each(["http://example.com/", "https://example.com/", "mailto:a@b.com"])(
    "opens allowed scheme: %s",
    async (url) => {
      await platform.openUrl(url);
      expect(openUrlMock).toHaveBeenCalledWith(url);
    }
  );

  it.each([
    "file:///etc/passwd",
    "javascript:alert(1)",
    "data:text/html,<script>alert(1)</script>",
  ])("refuses disallowed scheme: %s", async (url) => {
    await platform.openUrl(url);
    expect(openUrlMock).not.toHaveBeenCalled();
  });
});
