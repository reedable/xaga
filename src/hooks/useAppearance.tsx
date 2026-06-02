import { useContext } from "react";
import AppearanceContext from "../providers/AppearanceContext";

export function useAppearance() {
  const context = useContext(AppearanceContext);

  if (!context) {
    throw new TypeError("useView must be used within AppearanceProvider");
  }

  return {
    htmlReaderView: context.htmlReaderView,
    changeHtmlReaderView: context.changeHtmlReaderView,
    themeSetting: context.themeSetting,
    changeTheme: context.changeThemeSetting,
    themeMode: context.themeMode,
    htmlReaderFontFamily: context.htmlReaderFontFamily,
    setHtmlReaderFontFamily: context.setHtmlReaderFontFamily,
    htmlReaderFontSize: context.htmlReaderFontSize,
    setHtmlReaderFontSize: context.setHtmlReaderFontSize,
    htmlReaderLineHeight: context.htmlReaderLineHeight,
    setHtmlReaderLineHeight: context.setHtmlReaderLineHeight,
    showSidebar: context.showSidebar,
    setShowSidebar: context.setShowSidebar,
  };
}
