import { useEffect, useRef } from "react";
import useAbortController from "./useAbortController";
import { useFeeds } from "./useFeeds";
import { useSettings } from "./useSettings";
import { usePlatform } from "../platform/Platform";
import { useErrorSink } from "./useErrorSink";

/** One startup pass, using the same outcome presentation as manual refresh. */
export function useStartupFeedRefresh() {
  const { settingsLoaded, refreshOnStartup } = useSettings();
  const [, controller] = useAbortController();
  const { refreshAllFeeds } = useFeeds(controller);
  const platform = usePlatform();
  const { pushError } = useErrorSink();
  const started = useRef(false);
  useEffect(() => {
    if (!settingsLoaded || started.current) return;
    started.current = true;
    if (!refreshOnStartup) return;
    let disposed = false;
    void platform
      .isLibraryReadOnly()
      .then((readOnly) => {
        if (!disposed && !readOnly) return refreshAllFeeds();
      })
      .catch((error) => {
        if (!disposed) pushError(error);
      });
    return () => {
      disposed = true;
    };
  }, [settingsLoaded, refreshOnStartup, platform, refreshAllFeeds, pushError]);
}
