import { getCurrentWindow } from "@tauri-apps/api/window";
import { createRoot } from "react-dom/client";
import ErrorSinkProvider from "./providers/ErrorSinkProvider";
import App from "./App";
import "./index.css";
import { PlatformContext } from "./platform/Platform";
import { TauriPlatform } from "./platform/TauriPlatform";
import { AppearanceProvider } from "./providers/AppearanceProvider";
import { SettingsProvider } from "./providers/SettingsProvider";
import { decorate, logged } from "./utils/Decorator";
import Logger from "./utils/Logger";

const root = document.getElementById("root");
const platform = new TauriPlatform();

const logger = Logger.getLogger("history");

const _push = history.pushState.bind(history);
history.pushState = decorate(_push).with(logged, {
  logger,
  name: "pushState",
}) as typeof history.pushState;

const _replace = history.replaceState.bind(history);
history.replaceState = decorate(_replace).with(logged, {
  logger,
  name: "replaceState",
}) as typeof history.replaceState;

window.addEventListener("popstate", () =>
  logger.debug("popstate", location.pathname)
);

createRoot(root!).render(
  // <StrictMode>
  <PlatformContext.Provider value={platform}>
    <ErrorSinkProvider subscribeErrors={platform.onError}>
      <SettingsProvider>
        <AppearanceProvider>
          <App entryPoint={getCurrentWindow().label} />
        </AppearanceProvider>
      </SettingsProvider>
    </ErrorSinkProvider>
  </PlatformContext.Provider>
  // </StrictMode>
);
