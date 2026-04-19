import { useContext } from "react";
import AppearanceContext from "../providers/AppearanceContext";

export function useView() {
  const context = useContext(AppearanceContext);

  if (!context) {
    throw new TypeError("useView must be used within ViewProvider");
  }

  return {
    view: context.readerView,
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
