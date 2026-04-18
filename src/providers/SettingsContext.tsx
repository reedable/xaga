import { createContext } from "react";

export type SettingsContextType = {
  library: string;
  switchLibrary: () => Promise<void>;
  refreshOnStartup: boolean;
  setRefreshOnStartup: (value: boolean) => void;
  settingsLoaded: boolean;
};

const SettingsContext = createContext<SettingsContextType | null>(null);

export default SettingsContext;
