import { createTheme, CssBaseline, ThemeProvider } from "@mui/material";
import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { useSettings } from "../hooks/useSettings";
import { useErrorSink } from "../hooks/useErrorSink";
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

export function initialThemeSetting(): ThemeSetting {
  const value = (window as Window & { __XAGA_THEME_SETTING__?: unknown })
    .__XAGA_THEME_SETTING__;
  return value === "light" || value === "dark" || value === "auto"
    ? value
    : "auto";
}

export function AppearanceProvider({ children }: { children: ReactNode }) {
  const platform = usePlatform();
  const { saveSettings, refreshVersion } = useSettings();
  const { pushError } = useErrorSink();
  const [systemDark, setSystemDark] = useState(systemPrefersDark);
  const [themeSetting, setThemeSetting] =
    useState<ThemeSetting>(initialThemeSetting);

  const [htmlReaderView, setHtmlReaderView] =
    useState<HtmlReaderView>("reader");

  const [htmlReaderFontFamily, setHtmlReaderFontFamilyState] =
    useState<FontFamily>("system");

  const [htmlReaderFontSize, setHtmlReaderFontSizeState] = useState(
    HTML_READER_FONT_SIZE_DEFAULT
  );

  const [htmlReaderLineHeight, setHtmlReaderLineHeightState] = useState(
    HTML_READER_LINE_HEIGHT_DEFAULT
  );

  const [showSidebar, setShowSidebarState] = useState(true);

  const themeMode: ThemeMode =
    themeSetting === "auto" ? (systemDark ? "dark" : "light") : themeSetting;

  const muiTheme = createTheme({
    palette: { mode: themeMode },
  });

  const changeThemeSetting = useCallback(
    (t: ThemeSetting) => {
      void saveSettings(() => platform.setTheme(t)).then((saved) => {
        if (saved) setThemeSetting(t);
      });
    },
    [platform, saveSettings]
  );

  const changeHtmlReaderView = useCallback(
    (v: HtmlReaderView) => {
      void saveSettings(() => platform.setHtmlReaderView(v)).then((saved) => {
        if (saved) setHtmlReaderView(v);
      });
    },
    [platform, saveSettings]
  );

  const setHtmlReaderFontFamily = useCallback(
    (value: FontFamily) => {
      void saveSettings(() => platform.setHtmlReaderFontFamily(value)).then((saved) => {
        if (saved) setHtmlReaderFontFamilyState(value);
      });
    },
    [platform, saveSettings]
  );

  const setHtmlReaderFontSize = useCallback(
    (value: number) => {
      void saveSettings(() => platform.setHtmlReaderFontSize(value)).then((saved) => {
        if (saved) setHtmlReaderFontSizeState(value);
      });
    },
    [platform, saveSettings]
  );

  const setHtmlReaderLineHeight = useCallback(
    (value: number) => {
      void saveSettings(() => platform.setHtmlReaderLineHeight(value)).then((saved) => {
        if (saved) setHtmlReaderLineHeightState(value);
      });
    },
    [platform, saveSettings]
  );

  const setShowSidebar = useCallback(
    (value: boolean) => {
      void saveSettings(() => platform.setShowSidebar(value)).then((saved) => {
        if (saved) setShowSidebarState(value);
      });
    },
    [platform, saveSettings]
  );

  // Load persisted theme on startup.
  useEffect(() => {
    platform
      .getTheme()
      .then((t) => setThemeSetting(t as ThemeSetting))
      .catch(pushError);
  }, [platform, pushError, refreshVersion]);

  // Load persisted show_sidebar on startup.
  useEffect(() => {
    platform.getShowSidebar().then(setShowSidebarState).catch(pushError);
  }, [platform, pushError, refreshVersion]);

  // Load persisted html reader view on startup.
  useEffect(() => {
    platform
      .getHtmlReaderView()
      .then((v) => setHtmlReaderView(v as HtmlReaderView))
      .catch(pushError);
  }, [platform, pushError, refreshVersion]);

  // Load persisted html reader prefs on startup.
  useEffect(() => {
    platform
      .getHtmlReaderPrefs()
      .then(({ fontFamily, fontSize, lineHeight }) => {
        setHtmlReaderFontFamilyState(fontFamily as FontFamily);
        setHtmlReaderFontSizeState(fontSize);
        setHtmlReaderLineHeightState(lineHeight);
      })
      .catch(pushError);
  }, [platform, pushError, refreshVersion]);

  // Keep data-theme attribute in sync for CSS consumers.
  useEffect(() => {
    document.documentElement.setAttribute("data-theme", themeMode);
    document.documentElement.style.colorScheme = themeMode;
    document.documentElement.style.backgroundColor =
      themeMode === "dark" ? "#121212" : "#ffffff";
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
      platform.onHtmlReaderView((v) => changeHtmlReaderView(v)),
      platform.onShowSidebar((show) => setShowSidebar(show)),
    ];

    return () => {
      unlistens.forEach((p) => p.then((fn) => fn()));
    };
  }, [platform, changeThemeSetting, changeHtmlReaderView, setShowSidebar]);

  // Settings window sync — update state directly (Rust already persisted the value).
  useEffect(() => {
    const unlistens = [
      platform.onThemeChangedFromSettings((t) =>
        setThemeSetting(t as ThemeSetting)
      ),
      platform.onHtmlReaderViewChangedFromSettings((v) => setHtmlReaderView(v)),
      platform.onHtmlReaderFontFamilyChanged((v) =>
        setHtmlReaderFontFamilyState(v as FontFamily)
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
      changeHtmlReaderView,
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
      changeHtmlReaderView,
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
    ]
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
