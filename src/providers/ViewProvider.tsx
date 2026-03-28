import { CssBaseline, ThemeProvider } from "@mui/material";
import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { usePlatform } from "../platform/Platform";
import { darkTheme, lightTheme } from "../theme";
import type { View } from "../types";
import ViewContext, { type ThemeMode, type ThemeSetting } from "./ViewContext";

function systemPrefersDark(): boolean {
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

export function ViewProvider({ children }: { children: ReactNode }) {
  const platform = usePlatform();
  const [view, setView] = useState<View>("full");
  const [themeSetting, setThemeSetting] = useState<ThemeSetting>("auto");
  const [systemDark, setSystemDark] = useState(systemPrefersDark);

  const effectiveThemeMode: ThemeMode =
    themeSetting === "auto" ? (systemDark ? "dark" : "light") : themeSetting;

  const muiTheme = effectiveThemeMode === "dark" ? darkTheme : lightTheme;

  const changeThemeSetting = useCallback(
    (t: ThemeSetting) => {
      platform.setTheme(t).catch(console.error);
      setThemeSetting(t);
    },
    [platform],
  );

  // Load persisted theme on startup.
  useEffect(() => {
    platform
      .getTheme()
      .then((t) => setThemeSetting(t as ThemeSetting))
      .catch(console.error);
  }, [platform]);

  // Keep data-theme attribute in sync for CSS consumers.
  useEffect(() => {
    document.documentElement.setAttribute("data-theme", effectiveThemeMode);
  }, [effectiveThemeMode]);

  // Track system color-scheme changes so Auto mode reacts in real time.
  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (e: MediaQueryListEvent) => setSystemDark(e.matches);
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, []);

  // Menu event subscriptions.
  useEffect(() => {
    const unlistens = [
      platform.onSetTheme((t) => changeThemeSetting(t as ThemeSetting)),
      platform.onShowArticleSummary((showArticleSummary) =>
        setView(showArticleSummary ? "summary" : "full"),
      ),
    ];

    return () => {
      unlistens.forEach((p) => p.then((fn) => fn()));
    };
  }, [platform, changeThemeSetting]);

  const value = useMemo(
    () => ({
      view,
      setView,
      themeSetting,
      changeThemeSetting,
      themeMode: effectiveThemeMode,
    }),
    [view, themeSetting, changeThemeSetting, effectiveThemeMode],
  );

  return (
    <ViewContext.Provider value={value}>
      <ThemeProvider theme={muiTheme}>
        <CssBaseline />
        {children}
      </ThemeProvider>
    </ViewContext.Provider>
  );
}
