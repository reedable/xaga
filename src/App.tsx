import { useEffect, useState } from "react";
import { NavigationProvider } from "./providers/NavigationProvider";
import {
  BrowserRouter,
  MemoryRouter,
  Navigate,
  Route,
  Routes,
} from "react-router-dom";
import { isCaptureMode } from "./platform/captureMode";
import { CaptureReady } from "./platform/capture/CaptureReady";
import css from "./App.module.css";
import { AppLayout } from "./AppLayout";
import { Settings } from "./components/settings/Settings";
import { useEmbeddingStartupState } from "./hooks/useEmbeddingStartupState";
import { useSettings } from "./hooks/useSettings";
import { usePlatform } from "./platform/Platform";
import { ExpandedListProvider } from "./providers/ExpandedListProvider";
import { AllEntitiesCountsProvider } from "./providers/AllEntitiesCountsProvider";
import PinsProvider from "./providers/PinsProvider";
import FeedsProvider from "./providers/FeedsProvider";
import { TopicsProvider } from "./providers/TopicsProvider";
import { TagsProvider } from "./providers/TagsProvider";
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
          <NavigationProvider>
            <ExpandedListProvider>
              <PinsProvider>
                <TopicsProvider>
                  <FeedsProvider>
                    <SitesProvider
                      enabled={embeddingStartupState === "available"}
                    >
                      <TagsProvider
                        enabled={embeddingStartupState === "available"}
                      >
                        <Routes>
                          <Route
                            path="/"
                            element={
                              embeddingStartupState !== "loading" ? (
                                <AllEntitiesCountsProvider>
                                  <AppLayout />
                                  {isCaptureMode && <CaptureReady />}
                                </AllEntitiesCountsProvider>
                              ) : null
                            }
                          />
                          <Route
                            path="*"
                            element={<Navigate to="/" replace />}
                          />
                        </Routes>
                      </TagsProvider>
                    </SitesProvider>
                  </FeedsProvider>
                </TopicsProvider>
              </PinsProvider>
            </ExpandedListProvider>
          </NavigationProvider>
        </BrowserRouter>
      )}
    </>
  );
}
