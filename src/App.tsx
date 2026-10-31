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
import { useEmbeddingStartupState } from "./hooks/useEmbeddingStartupState";
import { useSettings } from "./hooks/useSettings";
import { usePlatform } from "./platform/Platform";
import { ExpandedListProvider } from "./providers/ExpandedListProvider";
import { AllItemsCountsProvider } from "./providers/AllItemsCountsProvider";
import PinsProvider from "./providers/PinsProvider";
import FeedsProvider from "./providers/FeedsProvider";
import { TopicsProvider } from "./providers/TopicsProvider";
import { SitesProvider } from "./providers/SitesProvider";

export default function App({ entryPoint }: { entryPoint: string }) {
  const platform = usePlatform();
  const { library } = useSettings();
  const embeddingStartupState = useEmbeddingStartupState();
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
        <BrowserRouter>
          <ExpandedListProvider>
            <PinsProvider>
              <TopicsProvider>
                <FeedsProvider>
                  <SitesProvider
                    enabled={embeddingStartupState === "available"}
                  >
                    <Routes>
                      <Route
                        path="/"
                        element={
                          embeddingStartupState !== "loading" ? (
                            <AllItemsCountsProvider>
                              <AppLayout />
                            </AllItemsCountsProvider>
                          ) : null
                        }
                      />
                      <Route path="*" element={<Navigate to="/" replace />} />
                    </Routes>
                  </SitesProvider>
                </FeedsProvider>
              </TopicsProvider>
            </PinsProvider>
          </ExpandedListProvider>
        </BrowserRouter>
      )}
    </>
  );
}
