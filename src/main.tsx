import { getCurrentWindow } from "@tauri-apps/api/window";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./index.css";
import { PlatformContext } from "./platform/Platform";
import { TauriPlatform } from "./platform/TauriPlatform";
import { AppearanceProvider } from "./providers/AppearanceProvider";
import { SettingsProvider } from "./providers/SettingsProvider";

const root = document.getElementById("root");
const platform = new TauriPlatform();

createRoot(root!).render(
  // <StrictMode>
  <PlatformContext.Provider value={platform}>
    <SettingsProvider>
      <AppearanceProvider>
        <App entryPoint={getCurrentWindow().label} />
      </AppearanceProvider>
    </SettingsProvider>
  </PlatformContext.Provider>
  // </StrictMode>
);
