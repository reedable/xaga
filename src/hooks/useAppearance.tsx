import { useContext } from "react";
import AppearanceContext from "../providers/AppearanceContext";

export function useAppearance() {
  const context = useContext(AppearanceContext);

  if (!context) {
    throw new TypeError("useView must be used within ViewProvider");
  }

  return {
    readerView: context.readerView,
    showOriginalSite: context.showOriginalSite,
    setView: context.setReaderView,
    themeSetting: context.themeSetting,
    changeTheme: context.changeThemeSetting,
    themeMode: context.themeMode,
    readerFontFamily: context.readerFontFamily,
    setReaderFontFamily: context.setReaderFontFamily,
    readerFontSize: context.readerFontSize,
    setReaderFontSize: context.setReaderFontSize,
    readerLineHeight: context.readerLineHeight,
    setReaderLineHeight: context.setReaderLineHeight,
  };
}
