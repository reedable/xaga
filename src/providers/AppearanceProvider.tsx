import { createTheme, CssBaseline, ThemeProvider } from "@mui/material";
import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { usePlatform } from "../platform/Platform";
import type { ReaderView } from "../types";
import type { FontFamily } from "../utils/FontFamily";
import AppearanceContext, {
  READER_FONT_SIZE_DEFAULT,
  READER_LINE_HEIGHT_DEFAULT,
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

  const [readerView, setReaderView] = useState<ReaderView>("full");
  const [showOriginalSite, setShowOriginalSite] = useState(false);

  const [readerFontFamily, setReaderFontFamilyState] =
    useState<FontFamily>("system");

  const [readerFontSize, setReaderFontSizeState] = useState(
    READER_FONT_SIZE_DEFAULT,
  );

  const [readerLineHeight, setReaderLineHeightState] = useState(
    READER_LINE_HEIGHT_DEFAULT,
  );

  const themeMode: ThemeMode =
    themeSetting === "auto" ? (systemDark ? "dark" : "light") : themeSetting;

  const muiTheme = createTheme({
    palette: { mode: themeMode },
    // TODO Consider adding systemFontFamily and systemFontSize
    // typography: {
    //   fontFamily: readerFontFamily,
    //   fontSize: readerFontSize,
    // },
  });

  const changeThemeSetting = useCallback(
    (t: ThemeSetting) => {
      platform.setTheme(t).catch(console.error);
      setThemeSetting(t);
    },
    [platform],
  );

  const setReaderFontFamily = useCallback(
    (value: FontFamily) => {
      platform.setReaderFontFamily(value).catch(console.error);
      setReaderFontFamilyState(value);
    },
    [platform],
  );

  const setReaderFontSize = useCallback(
    (value: number) => {
      platform.setReaderFontSize(value).catch(console.error);
      setReaderFontSizeState(value);
    },
    [platform],
  );

  const setReaderLineHeight = useCallback(
    (value: number) => {
      platform.setReaderLineHeight(value).catch(console.error);
      setReaderLineHeightState(value);
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

  // Load persisted reader prefs on startup.
  useEffect(() => {
    platform
      .getReaderPrefs()
      .then(({ fontFamily, fontSize, lineHeight }) => {
        setReaderFontFamilyState(fontFamily as FontFamily);
        setReaderFontSizeState(fontSize);
        setReaderLineHeightState(lineHeight);
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
        setReaderView(showArticleSummary ? "summary" : "full"),
      ),
      platform.onShowOriginalSite(setShowOriginalSite),
    ];

    return () => {
      unlistens.forEach((p) => p.then((fn) => fn()));
    };
  }, [platform, changeThemeSetting]);

  // Settings window sync — update state directly (Rust already persisted the value).
  useEffect(() => {
    const unlistens = [
      platform.onThemeChangedFromSettings((t) =>
        setThemeSetting(t as ThemeSetting),
      ),
      platform.onReaderFontFamilyChanged((v) =>
        setReaderFontFamilyState(v as FontFamily),
      ),
      platform.onReaderFontSizeChanged(setReaderFontSizeState),
      platform.onReaderLineHeightChanged(setReaderLineHeightState),
    ];

    return () => {
      unlistens.forEach((p) => p.then((fn) => fn()));
    };
  }, [platform]);

  const value = useMemo(
    () => ({
      readerView,
      setReaderView,
      showOriginalSite,
      themeSetting,
      changeThemeSetting,
      themeMode,
      readerFontFamily,
      setReaderFontFamily,
      readerFontSize,
      setReaderFontSize,
      readerLineHeight,
      setReaderLineHeight,
    }),
    [
      readerView,
      showOriginalSite,
      themeSetting,
      changeThemeSetting,
      themeMode,
      readerFontFamily,
      setReaderFontFamily,
      readerFontSize,
      setReaderFontSize,
      readerLineHeight,
      setReaderLineHeight,
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
