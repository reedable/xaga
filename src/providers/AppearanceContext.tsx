import { createContext } from "react";
import type { ReaderView } from "../types";
import type { FontFamily } from "../utils/FontFamily";

export type ThemeMode = "light" | "dark";
export type ThemeSetting = "auto" | ThemeMode;

export const READER_FONT_SIZE_MIN = 12;
export const READER_FONT_SIZE_MAX = 24;
export const READER_FONT_SIZE_DEFAULT = 16;
export const READER_LINE_HEIGHT_MIN = 1.2;
export const READER_LINE_HEIGHT_MAX = 2.0;
export const READER_LINE_HEIGHT_STEP = 0.1;
export const READER_LINE_HEIGHT_DEFAULT = 1.75;

export type AppearanceContextType = {
  readerView: ReaderView;
  setReaderView: (readerView: ReaderView) => void;
  showOriginalSite: boolean;
  themeSetting: ThemeSetting;
  changeThemeSetting: (mode: ThemeSetting) => void;
  themeMode: ThemeMode;
  readerFontFamily: FontFamily;
  setReaderFontFamily: (value: FontFamily) => void;
  readerFontSize: number;
  setReaderFontSize: (value: number) => void;
  readerLineHeight: number;
  setReaderLineHeight: (value: number) => void;
};

const AppearanceContext = createContext<AppearanceContextType | null>(null);

export default AppearanceContext;
