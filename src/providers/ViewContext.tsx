import { createContext } from "react";
import type { View } from "../types";

export type ThemeMode = "light" | "dark";
export type ThemeSetting = "auto" | ThemeMode;

export type FontFamily = "system" | "serif" | "sans-serif" | "monospace";

export const FONT_FAMILY_CSS: Record<FontFamily, string> = {
  system: 'Roboto,"Helvetica Neue",Arial,sans-serif',
  serif: 'Georgia,"Times New Roman",serif',
  "sans-serif": '"Helvetica Neue",Helvetica,Arial,sans-serif',
  monospace: '"Courier New",Courier,monospace',
};

export const READER_FONT_SIZE_MIN = 12;
export const READER_FONT_SIZE_MAX = 24;
export const READER_FONT_SIZE_DEFAULT = 16;
export const READER_LINE_HEIGHT_MIN = 1.2;
export const READER_LINE_HEIGHT_MAX = 2.0;
export const READER_LINE_HEIGHT_STEP = 0.1;
export const READER_LINE_HEIGHT_DEFAULT = 1.75;

export type ViewContextType = {
  view: View;
  setView: (view: View) => void;
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

const ViewContext = createContext<ViewContextType | null>(null);

export default ViewContext;
