import { useEffect, useState } from "react";
import {
  BrowserRouter,
  MemoryRouter,
  Navigate,
  Route,
  Routes,
} from "react-router-dom";
import css from "./App.module.css";
import { AppLayout } from "./AppLayout";
import { Settings } from "./components/settings/Settings";
import { useCacheReady } from "./hooks/useCacheReady";
import { useSettings } from "./hooks/useSettings";
import { usePlatform } from "./platform/Platform";
import ErrorSinkProvider from "./providers/ErrorSinkProvider";
import { ExpandedListProvider } from "./providers/ExpandedListProvider";
import FeedsProvider from "./providers/FeedsProvider";
import { TopicsProvider } from "./providers/TopicsProvider";

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
    <ErrorSinkProvider subscribeErrors={platform.onError}>
      {shuttingDown && (
        <div className={css.ShutdownOverlay}>Saving data, please wait…</div>
      )}

      {entryPoint === "settings" ? (
        <MemoryRouter initialEntries={["/general"]}>
          <Settings />
        </MemoryRouter>
      ) : (
        <BrowserRouter>
          <ExpandedListProvider>
            <TopicsProvider>
              <FeedsProvider>
                <Routes>
                  <Route
                    path="/"
                    element={cacheReady ? <AppLayout /> : null}
                  />
                  <Route path="*" element={<Navigate to="/" replace />} />
                </Routes>
              </FeedsProvider>
            </TopicsProvider>
          </ExpandedListProvider>
        </BrowserRouter>
      )}
    </ErrorSinkProvider>
  );
}
