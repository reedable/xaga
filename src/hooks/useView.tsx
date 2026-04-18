import { useContext } from "react";
import ViewContext from "../providers/ViewContext";

export function useView() {
  const context = useContext(ViewContext);

  if (!context) {
    throw new TypeError("useView must be used within ViewProvider");
  }

  return {
    view: context.view,
    setView: context.setView,
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
