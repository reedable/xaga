import { createContext } from "react";

export type SettingsContextType = {
  library: string;
  refreshVersion: number;
  saveSettings: (write: () => Promise<void>) => Promise<boolean>;
  switchLibrary: () => Promise<void>;
  refreshOnStartup: boolean;
  setRefreshOnStartup: (value: boolean) => void;
  inlinePdfReader: boolean;
  setInlinePdfReader: (value: boolean) => void;
  settingsLoaded: boolean;
};

const SettingsContext = createContext<SettingsContextType | null>(null);

export default SettingsContext;
