import { useEffect, useState } from "react";
import { usePlatform } from "../platform/Platform";
import { subscribe } from "./useEvents";

/**
 * Whether this instance is read-only for the open library (its version does
 * not match, its upgrade is incomplete, or no library write lock is held). Same name as the Rust accessor, the Tauri
 * command, and the `library:mode-changed` event payload — one concept, one
 * name at every layer.
 *
 * Initial value comes from the command; the event covers the startup race
 * (mode is announced once at setup and is fixed for the process's lifetime —
 * a library switch restarts the process). Defaults to `false` (writable)
 * until the first answer arrives, so write affordances don't flash disabled
 * on every launch; a read-only instance's writes are refused by the backend
 * gate regardless of what the UI shows.
 */
export function useLibraryReadOnly(): boolean {
  const platform = usePlatform();
  const [isLibraryReadOnly, setLibraryReadOnly] = useState(false);

  useEffect(() => {
    let cancelled = false;

    platform.isLibraryReadOnly().then((value) => {
      if (!cancelled) setLibraryReadOnly(value);
    });
    const unsubscribe = subscribe(() =>
      platform.onLibraryModeChanged((value) => setLibraryReadOnly(value))
    );

    return () => {
      cancelled = true;
      unsubscribe();
    };
  }, [platform]);

  return isLibraryReadOnly;
}
