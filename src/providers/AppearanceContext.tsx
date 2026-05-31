import { createContext } from "react";
import type { HtmlReaderView } from "../types";
import type { FontFamily } from "../utils/FontFamily";

export type ThemeMode = "light" | "dark";
export type ThemeSetting = "auto" | ThemeMode;

export const HTML_READER_FONT_SIZE_MIN = 12;
export const HTML_READER_FONT_SIZE_MAX = 24;
export const HTML_READER_FONT_SIZE_DEFAULT = 16;
export const HTML_READER_LINE_HEIGHT_MIN = 1.2;
export const HTML_READER_LINE_HEIGHT_MAX = 2.0;
export const HTML_READER_LINE_HEIGHT_STEP = 0.1;
export const HTML_READER_LINE_HEIGHT_DEFAULT = 1.75;

export type AppearanceContextType = {
  htmlReaderView: HtmlReaderView;
  setHtmlReaderView: (htmlReaderView: HtmlReaderView) => void;
  themeSetting: ThemeSetting;
  changeThemeSetting: (mode: ThemeSetting) => void;
  themeMode: ThemeMode;
  htmlReaderFontFamily: FontFamily;
  setHtmlReaderFontFamily: (value: FontFamily) => void;
  htmlReaderFontSize: number;
  setHtmlReaderFontSize: (value: number) => void;
  htmlReaderLineHeight: number;
  setHtmlReaderLineHeight: (value: number) => void;
  showSidebar: boolean;
  setShowSidebar: (value: boolean) => void;
};

const AppearanceContext = createContext<AppearanceContextType | null>(null);

export default AppearanceContext;
