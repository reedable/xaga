import { useEffect, useState } from "react";
import { MemoryRouter, Navigate, Route, Routes } from "react-router-dom";
import css from "./_.module.css";
import { AppLayout } from "./AppLayout";
import { Settings } from "./components/settings/Settings";
import { useCacheReady } from "./hooks/useCacheReady";
import { useSettings } from "./hooks/useSettings";
import { usePlatform } from "./platform/Platform";
import FeedsProvider from "./providers/FeedsProvider";
import { ManageFeedProvider } from "./providers/ManageFeedProvider";
import { ManageTopicProvider } from "./providers/ManageTopicProvider";
import { ALL_ITEMS } from "./Shortcuts";
import { isFilter } from "./types";

const storedFilter = (() => {
  try {
    const filter = localStorage.getItem("list.filter");
    return isFilter(filter) ? filter : "unread";
  } catch {
    return "unread";
  }
})();

const initialEntry = `/${ALL_ITEMS.id}?filter=${storedFilter}`;

export default function App({ entryPoint }: { entryPoint: string }) {
  const platform = usePlatform();
  const { library } = useSettings();
  const cacheReady = useCacheReady();
  const [shuttingDown, setShuttingDown] = useState(false);

  useEffect(() => {
    const name = library ? library.split("/").pop() || "Xaga" : null;
    const title = name ? `Xaga — ${name}` : "Xaga";
    platform.setWindowTitle(title);
  }, [library, platform]);

  useEffect(() => {
    const unlisten = platform.onShutdown(() => setShuttingDown(true));
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [platform]);

  return (
    <>
      {shuttingDown && (
        <div className={css.ShutdownOverlay}>Saving data, please wait…</div>
      )}

      {entryPoint === "settings" ? (
        <MemoryRouter initialEntries={["/general"]}>
          <Settings />
        </MemoryRouter>
      ) : (
        <MemoryRouter initialEntries={[initialEntry]}>
          <ManageTopicProvider>
            <FeedsProvider>
              <ManageFeedProvider>
                <Routes>
                  <Route
                    path="/:collectionId/:itemId"
                    element={cacheReady ? <AppLayout /> : null}
                  />
                  <Route
                    path="/:collectionId"
                    element={cacheReady ? <AppLayout /> : null}
                  />
                  <Route
                    path="*"
                    element={<Navigate to={initialEntry} replace />}
                  />
                </Routes>
              </ManageFeedProvider>
            </FeedsProvider>
          </ManageTopicProvider>
        </MemoryRouter>
      )}
    </>
  );
}
