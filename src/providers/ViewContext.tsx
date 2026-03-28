import { createContext } from "react";
import type { View } from "../types";

export type ThemeMode = "light" | "dark";
export type ThemeSetting = "auto" | ThemeMode;

export type ViewContextType = {
  view: View;
  setView: (view: View) => void;
  themeSetting: ThemeSetting;
  changeThemeSetting: (mode: ThemeSetting) => void;
  themeMode: ThemeMode;
};

const ViewContext = createContext<ViewContextType | null>(null);

export default ViewContext;
