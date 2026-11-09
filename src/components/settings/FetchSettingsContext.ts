import { createContext } from "react";
import type {
  FetchSettings,
  FetchSettingChange,
} from "../../platform/fetchSettings";

export const FetchSettingsContext = createContext<{
  settings: FetchSettings | null;
  save: (change: FetchSettingChange) => Promise<boolean>;
} | null>(null);
