import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { usePlatform } from "../platform/Platform";
import Logger from "../utils/Logger";
import SettingsContext from "./SettingsContext";

const logger = Logger.getLogger("SettingsProvider");

export function SettingsProvider({ children }: { children: ReactNode }) {
  const platform = usePlatform();
  const [library, setLibrary] = useState<string>("");
  const [refreshOnStartup, setRefreshOnStartupState] = useState<boolean>(false);
  const [inlinePdfReader, setInlinePdfReaderState] = useState<boolean>(false);
  const [settingsLoaded, setSettingsLoaded] = useState<boolean>(false);

  const switchLibrary = useCallback(async () => {
    const selected = await platform.openFileDialog(true, false);
    if (typeof selected === "string") {
      const canonical = await platform.setLibrary(selected);
      setLibrary(canonical);
    }
  }, [platform]);

  const setRefreshOnStartup = useCallback(
    (value: boolean) => {
      platform.setRefreshOnStartup(value).catch(logger.error);
      setRefreshOnStartupState(value);
    },
    [platform]
  );

  const setInlinePdfReader = useCallback(
    (value: boolean) => {
      platform.setInlinePdfReader(value).catch(logger.error);
      setInlinePdfReaderState(value);
    },
    [platform]
  );

  useEffect(() => {
    Promise.all([
      platform.getLibrary().then(setLibrary),
      platform.getRefreshOnStartup().then(setRefreshOnStartupState),
      platform.getInlinePdfReader().then(setInlinePdfReaderState),
    ])
      .catch(logger.error)
      .finally(() => setSettingsLoaded(true));
  }, [platform]);

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
      library,
      switchLibrary,
      refreshOnStartup,
      setRefreshOnStartup,
      inlinePdfReader,
      setInlinePdfReader,
      settingsLoaded,
    }),
    [
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
