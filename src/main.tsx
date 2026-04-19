import { getCurrentWindow } from "@tauri-apps/api/window";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { MemoryRouter } from "react-router-dom";
import App from "./App.tsx";
import { Settings } from "./components/settings/Settings.tsx";
import "./index.css";
import { TauriPlatform } from "./platform/TauriPlatform.ts";
import { PlatformContext } from "./platform/Platform.ts";
import { AppearanceProvider } from "./providers/AppearanceProvider.tsx";
import { SettingsProvider } from "./providers/SettingsProvider.tsx";
import FeedsProvider from "./providers/FeedsProvider.tsx";

const platform = new TauriPlatform();
const isSettingsWindow = getCurrentWindow().label === "settings";

const root = createRoot(document.getElementById("root")!);

if (isSettingsWindow) {
  root.render(
    <StrictMode>
      <PlatformContext.Provider value={platform}>
        <AppearanceProvider>
          <SettingsProvider>
            <MemoryRouter initialEntries={["/general"]}>
              <Settings />
            </MemoryRouter>
          </SettingsProvider>
        </AppearanceProvider>
      </PlatformContext.Provider>
    </StrictMode>,
  );
} else {
  root.render(
    <StrictMode>
      <PlatformContext.Provider value={platform}>
        <AppearanceProvider>
          <SettingsProvider>
            <FeedsProvider>
              <MemoryRouter>
                <App />
              </MemoryRouter>
            </FeedsProvider>
          </SettingsProvider>
        </AppearanceProvider>
      </PlatformContext.Provider>
    </StrictMode>,
  );
}
