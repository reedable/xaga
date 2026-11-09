import { createContext } from "react";

export const SettingsSaveContext = createContext<Set<
  () => Promise<boolean>
> | null>(null);
