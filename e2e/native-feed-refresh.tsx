// Native integration fixture: all refresh and notification behavior is production code.
import { useRef, useState } from "react";
import { createRoot } from "react-dom/client";
/* oxlint-disable react/only-export-components -- Standalone native test entry point. */
import { getCurrentWindow } from "@tauri-apps/api/window";
import { PlatformContext } from "../src/platform/Platform";
import { TauriPlatform } from "../src/platform/TauriPlatform";
import ErrorSinkProvider from "../src/providers/ErrorSinkProvider";
import FeedsContext from "../src/providers/FeedsContext";
import { ExpandedListProvider } from "../src/providers/ExpandedListProvider";
import { useFeeds } from "../src/hooks/useFeeds";
import type { Feed } from "../src/types";

const platform = new TauriPlatform();
function Refresh() {
  const controller = useRef(new AbortController());
  const { refreshAllFeeds, loading } = useFeeds(controller);
  const [busy, setBusy] = useState(false);
  return (
    <button
      disabled={busy || loading}
      onClick={async () => {
        setBusy(true);
        try {
          await refreshAllFeeds();
        } finally {
          setBusy(false);
        }
      }}
    >
      Refresh
    </button>
  );
}
function Fixture() {
  const [error, setError] = useState<unknown>(null);
  const [loading, setLoading] = useState(false);
  const [loadingFeedIds, setLoadingSet] = useState(new Set<string>());
  const [feeds, setFeeds] = useState<Feed[]>([]);
  return (
    <PlatformContext.Provider value={platform}>
      <ErrorSinkProvider>
        <ExpandedListProvider>
          <FeedsContext.Provider
            value={{
              error,
              loading,
              loadingFeedIds,
              feeds,
              pinnedFeedIds: [],
              setError,
              setLoading,
              setLoadingSet,
              setFeeds,
              setFeedPinned: () => {},
            }}
          >
            <Refresh />
          </FeedsContext.Provider>
        </ExpandedListProvider>
      </ErrorSinkProvider>
    </PlatformContext.Provider>
  );
}
createRoot(document.getElementById("root")!).render(<Fixture />);
async function waitFor(predicate: () => unknown) {
  const deadline = Date.now() + 15000;
  while (!predicate()) {
    if (Date.now() > deadline)
      throw new Error(
        "Native UI condition timed out: " + document.body.innerText
      );
    await new Promise((resolve) => setTimeout(resolve, 25));
  }
}
let inspectedAlert = false;
function visibleAlert() {
  const alert = document.querySelector<HTMLElement>('[role="alert"]');
  if (!alert) return false;
  const style = getComputedStyle(alert);
  const bounds = alert.getBoundingClientRect();
  if (!inspectedAlert) {
    inspectedAlert = true;
    console.log(
      "Native alert rendering",
      JSON.stringify({
        hidden: document.hidden,
        visibility: style.visibility,
        opacity: style.opacity,
        width: bounds.width,
        height: bounds.height,
      })
    );
  }
  return (
    bounds.width > 0 &&
    bounds.height > 0 &&
    style.visibility === "visible" &&
    Number(style.opacity) > 0
  );
}
try {
  await waitFor(() => document.querySelector("button"));
  const refresh = document.querySelector("button")!;
  refresh.click();
  await waitFor(visibleAlert);
  const message = document.querySelector('[role="alert"]')!.textContent ?? "";
  if (
    !message.includes("Unavailable feed") ||
    !message.includes("Use Refresh to retry.")
  )
    throw new Error(message);
  await waitFor(() => !refresh.disabled);
  (document.querySelector('[aria-label="Close"]') as HTMLButtonElement).click();
  await waitFor(() => !document.querySelector('[role="alert"]'));
  refresh.click();
  await waitFor(() => !refresh.disabled);
  await waitFor(visibleAlert);
  await getCurrentWindow().setTitle("PASS: " + message);
} catch (error) {
  console.error("Native refresh verification", error);
  await getCurrentWindow().setTitle("FAIL: " + String(error));
}
