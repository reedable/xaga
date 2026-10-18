import type { ThemeSetting } from "./AppearanceContext";

export function initialThemeSetting(): ThemeSetting {
  const value = (window as Window & { __XAGA_THEME_SETTING__?: unknown })
    .__XAGA_THEME_SETTING__;
  return value === "light" || value === "dark" || value === "auto"
    ? value
    : "auto";
}
