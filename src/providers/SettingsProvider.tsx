import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { usePlatform } from "../platform/Platform";
import { useErrorSink } from "../hooks/useErrorSink";
import SettingsContext from "./SettingsContext";

export function SettingsProvider({ children }: { children: ReactNode }) {
  const platform = usePlatform();
  const { pushError } = useErrorSink();
  const [refreshVersion, setRefreshVersion] = useState(0);
  const saveSettings = useCallback(
    async (write: () => Promise<void>) => {
      try {
        await write();
        return true;
      } catch (error) {
        pushError(error);
        setRefreshVersion((version) => version + 1);
        return false;
      }
    },
    [pushError]
  );
  const [library, setLibrary] = useState<string>("");
  const [refreshOnStartup, setRefreshOnStartupState] = useState<boolean>(false);
  const [inlinePdfReader, setInlinePdfReaderState] = useState<boolean>(false);
  const [settingsLoaded, setSettingsLoaded] = useState<boolean>(false);

  const switchLibrary = useCallback(async () => {
    const selected = await platform.openFileDialog(true, false);
    if (typeof selected === "string") {
      await saveSettings(async () => {
        const canonical = await platform.setLibrary(selected);
        setLibrary(canonical);
      });
    }
  }, [platform, saveSettings]);

  const setRefreshOnStartup = useCallback(
    (value: boolean) => {
      void saveSettings(() => platform.setRefreshOnStartup(value)).then(
        (saved) => {
          if (saved) setRefreshOnStartupState(value);
        }
      );
    },
    [platform, saveSettings]
  );

  const setInlinePdfReader = useCallback(
    (value: boolean) => {
      void saveSettings(() => platform.setInlinePdfReader(value)).then(
        (saved) => {
          if (saved) setInlinePdfReaderState(value);
        }
      );
    },
    [platform, saveSettings]
  );

  useEffect(() => {
    Promise.all([
      platform.getLibrary().then(setLibrary),
      platform.getRefreshOnStartup().then(setRefreshOnStartupState),
      platform.getInlinePdfReader().then(setInlinePdfReaderState),
    ])
      .catch(pushError)
      .finally(() => setSettingsLoaded(true));
  }, [platform, pushError, refreshVersion]);

  useEffect(() => {
    const unlisten = platform.onInlinePdfReaderChanged(setInlinePdfReaderState);
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [platform]);

  useEffect(() => {
    const unlisten = platform.onOpenLibrary(switchLibrary);
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [platform, switchLibrary]);

  useEffect(() => {
    const unlisten = platform.onRefreshOnStartupChanged(
      setRefreshOnStartupState
    );
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [platform]);

  const value = useMemo(
    () => ({
      refreshVersion,
      saveSettings,
      library,
      switchLibrary,
      refreshOnStartup,
      setRefreshOnStartup,
      inlinePdfReader,
      setInlinePdfReader,
      settingsLoaded,
    }),
    [
      refreshVersion,
      saveSettings,
      library,
      switchLibrary,
      refreshOnStartup,
      setRefreshOnStartup,
      inlinePdfReader,
      setInlinePdfReader,
      settingsLoaded,
    ]
  );

  return (
    <SettingsContext.Provider value={value}>
      {children}
    </SettingsContext.Provider>
  );
}
