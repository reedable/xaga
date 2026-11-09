import { Alert } from "@mui/material";
import { useEffect, useState, type ReactNode } from "react";
import { usePlatform } from "../../platform/Platform";
import type {
  FetchSettings,
  FetchSettingChange,
} from "../../platform/fetchSettings";
import { useSettings } from "../../hooks/useSettings";
import { useErrorSink } from "../../hooks/useErrorSink";

import { FetchSettingsContext } from "./FetchSettingsContext";

export function FetchSettingsProvider({ children }: { children: ReactNode }) {
  const platform = usePlatform();
  const { refreshVersion, saveSettings } = useSettings();
  const { pushError } = useErrorSink();
  const [settings, setSettings] = useState<FetchSettings | null>(null);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    let active = true;
    platform
      .getFetchSettings()
      .then((value) => {
        if (active) {
          setSettings(value);
          setFailed(false);
        }
      })
      .catch((error) => {
        if (active) {
          setFailed(true);
          pushError(error);
        }
      });
    return () => {
      active = false;
    };
  }, [platform, refreshVersion, pushError]);

  async function save(change: FetchSettingChange) {
    const saved = await saveSettings(() => platform.setFetchSetting(change));
    if (saved)
      setSettings(
        (previous) =>
          previous && { ...previous, [change.setting]: change.value }
      );
    return saved;
  }
  return (
    <FetchSettingsContext.Provider value={{ settings, save }}>
      {failed && (
        <Alert severity="error">
          Could not load fetch settings. Reopen Settings to retry.
        </Alert>
      )}
      {children}
    </FetchSettingsContext.Provider>
  );
}
