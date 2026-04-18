import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { MemoryRouter } from "react-router-dom";
import App from "./App.tsx";
import "./index.css";
import { TauriPlatform } from "./platform/TauriPlatform.ts";
import { PlatformContext } from "./platform/Platform.ts";
import { ViewProvider } from "./providers/ViewProvider.tsx";
import { SettingsProvider } from "./providers/SettingsProvider.tsx";
import FeedsProvider from "./providers/FeedsProvider.tsx";

const platform = new TauriPlatform();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <PlatformContext.Provider value={platform}>
      <ViewProvider>
        <SettingsProvider>
          <FeedsProvider>
            <MemoryRouter>
              <App />
            </MemoryRouter>
          </FeedsProvider>
        </SettingsProvider>
      </ViewProvider>
    </PlatformContext.Provider>
  </StrictMode>,
);
