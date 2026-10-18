import { afterEach, describe, expect, it } from "vitest";
import { initialThemeSetting } from "./AppearanceProvider";

const bootstrapWindow = window as Window & {
  __XAGA_THEME_SETTING__?: unknown;
};

afterEach(() => {
  delete bootstrapWindow.__XAGA_THEME_SETTING__;
});

describe("initialThemeSetting", () => {
  it.each(["light", "dark", "auto"] as const)(
    "uses the native %s bootstrap value",
    (theme) => {
      bootstrapWindow.__XAGA_THEME_SETTING__ = theme;
      expect(initialThemeSetting()).toBe(theme);
    }
  );

  it("falls back to auto without a valid native bootstrap value", () => {
    bootstrapWindow.__XAGA_THEME_SETTING__ = "unexpected";
    expect(initialThemeSetting()).toBe("auto");
  });
});
