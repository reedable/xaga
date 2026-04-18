import { useContext } from "react";
import SettingsContext from "../providers/SettingsContext";

export function useSettings() {
  const context = useContext(SettingsContext);

  if (!context) {
    throw new TypeError("useSettings must be used within SettingsProvider");
  }

  return context;
}
