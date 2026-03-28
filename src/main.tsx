import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App.tsx";
import "./index.css";
import { TauriPlatform } from "./platform/TauriPlatform.ts";
import { PlatformContext } from "./platform/Platform.ts";
import { ViewProvider } from "./providers/ViewProvider.tsx";

const platform = new TauriPlatform();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <PlatformContext.Provider value={platform}>
      <ViewProvider>
        <App />
      </ViewProvider>
    </PlatformContext.Provider>
  </StrictMode>,
);
