import { createTheme, CssBaseline, ThemeProvider } from "@mui/material";
import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { usePlatform } from "../platform/Platform";
import type { HtmlReaderView } from "../types";
import type { FontFamily } from "../utils/FontFamily";
import AppearanceContext, {
  HTML_READER_FONT_SIZE_DEFAULT,
  HTML_READER_LINE_HEIGHT_DEFAULT,
  type ThemeMode,
  type ThemeSetting,
} from "./AppearanceContext";

function systemPrefersDark(): boolean {
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

export function AppearanceProvider({ children }: { children: ReactNode }) {
  const platform = usePlatform();
  const [systemDark, setSystemDark] = useState(systemPrefersDark);
  const [themeSetting, setThemeSetting] = useState<ThemeSetting>("auto");

  const [htmlReaderView, setHtmlReaderView] = useState<HtmlReaderView>("full");

  const [htmlReaderFontFamily, setHtmlReaderFontFamilyState] =
    useState<FontFamily>("system");

  const [htmlReaderFontSize, setHtmlReaderFontSizeState] = useState(
    HTML_READER_FONT_SIZE_DEFAULT,
  );

  const [htmlReaderLineHeight, setHtmlReaderLineHeightState] = useState(
    HTML_READER_LINE_HEIGHT_DEFAULT,
  );

  const [showSidebar, setShowSidebarState] = useState(true);

  const themeMode: ThemeMode =
    themeSetting === "auto" ? (systemDark ? "dark" : "light") : themeSetting;

  const muiTheme = createTheme({
    palette: { mode: themeMode },
  });

  const changeThemeSetting = useCallback(
    (t: ThemeSetting) => {
      platform.setTheme(t).catch(console.error);
      setThemeSetting(t);
    },
    [platform],
  );

  const setHtmlReaderFontFamily = useCallback(
    (value: FontFamily) => {
      platform.setHtmlReaderFontFamily(value).catch(console.error);
      setHtmlReaderFontFamilyState(value);
    },
    [platform],
  );

  const setHtmlReaderFontSize = useCallback(
    (value: number) => {
      platform.setHtmlReaderFontSize(value).catch(console.error);
      setHtmlReaderFontSizeState(value);
    },
    [platform],
  );

  const setHtmlReaderLineHeight = useCallback(
    (value: number) => {
      platform.setHtmlReaderLineHeight(value).catch(console.error);
      setHtmlReaderLineHeightState(value);
    },
    [platform],
  );

  const setShowSidebar = useCallback(
    (value: boolean) => {
      platform.setShowSidebar(value).catch(console.error);
      setShowSidebarState(value);
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

  // Load persisted show_sidebar on startup.
  useEffect(() => {
    platform
      .getShowSidebar()
      .then(setShowSidebarState)
      .catch(console.error);
  }, [platform]);

  // Load persisted html reader prefs on startup.
  useEffect(() => {
    platform
      .getHtmlReaderPrefs()
      .then(({ fontFamily, fontSize, lineHeight }) => {
        setHtmlReaderFontFamilyState(fontFamily as FontFamily);
        setHtmlReaderFontSizeState(fontSize);
        setHtmlReaderLineHeightState(lineHeight);
      })
      .catch(console.error);
  }, [platform]);

  // Keep data-theme attribute in sync for CSS consumers.
  useEffect(() => {
    document.documentElement.setAttribute("data-theme", themeMode);
  }, [themeMode]);

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
        setHtmlReaderView(showArticleSummary ? "summary" : "full"),
      ),
      platform.onShowSidebar((show) => setShowSidebar(show)),
    ];

    return () => {
      unlistens.forEach((p) => p.then((fn) => fn()));
    };
  }, [platform, changeThemeSetting, setShowSidebar]);

  // Settings window sync — update state directly (Rust already persisted the value).
  useEffect(() => {
    const unlistens = [
      platform.onThemeChangedFromSettings((t) =>
        setThemeSetting(t as ThemeSetting),
      ),
      platform.onHtmlReaderFontFamilyChanged((v) =>
        setHtmlReaderFontFamilyState(v as FontFamily),
      ),
      platform.onHtmlReaderFontSizeChanged(setHtmlReaderFontSizeState),
      platform.onHtmlReaderLineHeightChanged(setHtmlReaderLineHeightState),
    ];

    return () => {
      unlistens.forEach((p) => p.then((fn) => fn()));
    };
  }, [platform]);

  const value = useMemo(
    () => ({
      htmlReaderView,
      setHtmlReaderView,
      themeSetting,
      changeThemeSetting,
      themeMode,
      htmlReaderFontFamily,
      setHtmlReaderFontFamily,
      htmlReaderFontSize,
      setHtmlReaderFontSize,
      htmlReaderLineHeight,
      setHtmlReaderLineHeight,
      showSidebar,
      setShowSidebar,
    }),
    [
      htmlReaderView,
      themeSetting,
      changeThemeSetting,
      themeMode,
      htmlReaderFontFamily,
      setHtmlReaderFontFamily,
      htmlReaderFontSize,
      setHtmlReaderFontSize,
      htmlReaderLineHeight,
      setHtmlReaderLineHeight,
      showSidebar,
      setShowSidebar,
    ],
  );

  return (
    <AppearanceContext.Provider value={value}>
      <ThemeProvider theme={muiTheme}>
        <CssBaseline />
        {children}
      </ThemeProvider>
    </AppearanceContext.Provider>
  );
}
