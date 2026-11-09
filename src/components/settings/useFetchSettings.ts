import { useContext } from "react";
import { FetchSettingsContext } from "./FetchSettingsContext";

export function useFetchSettings() {
  const context = useContext(FetchSettingsContext);
  if (!context) throw new Error("Fetch settings require FetchSettingsProvider");
  return context;
}
