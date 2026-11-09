import { getCurrentWindow } from "@tauri-apps/api/window";
import { createRoot } from "react-dom/client";
import ErrorSinkProvider from "./providers/ErrorSinkProvider";
import App from "./App";
import "@reedable/lliwio-css/colors.css";
import "@reedable/lliwio-css/semanticColors.css";
import "./index.css";
import { PlatformContext } from "./platform/Platform";
import { isCaptureMode } from "./platform/captureMode";
import { AppearanceProvider } from "./providers/AppearanceProvider";
import { SettingsProvider } from "./providers/SettingsProvider";
import { EmbeddingStartupProvider } from "./providers/EmbeddingStartupProvider";
import { decorate, logged } from "./utils/Decorator";
import Logger from "./utils/Logger";

const root = document.getElementById("root");
const captureTheme =
  new URLSearchParams(location.search).get("theme") === "dark"
    ? "dark"
    : "light";
const platform =
  import.meta.env.DEV && import.meta.env.MODE === "capture"
    ? new (await import("./platform/capture/MockPlatform")).MockPlatform(
        captureTheme
      )
    : new (await import("./platform/TauriPlatform")).TauriPlatform();
const entryPoint = isCaptureMode
  ? new URLSearchParams(location.search).get("window") === "settings"
    ? "settings"
    : "main"
  : getCurrentWindow().label;
if (isCaptureMode) {
  const scenario = new URLSearchParams(location.search).get("scenario");
  history.replaceState(
    null,
    "",
    `/?${new URLSearchParams({ ...(scenario ? { scenario } : {}), ...(entryPoint === "settings" ? { window: "settings" } : {}) })}`
  );
  Object.assign(window, {
    xagaCapture: { reset: () => location.reload() },
    __XAGA_THEME_SETTING__: captureTheme,
  });
}

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
          <EmbeddingStartupProvider>
            <App entryPoint={entryPoint} />
          </EmbeddingStartupProvider>
        </AppearanceProvider>
      </SettingsProvider>
    </ErrorSinkProvider>
  </PlatformContext.Provider>
  // </StrictMode>
);
