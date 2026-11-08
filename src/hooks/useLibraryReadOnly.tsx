import { useEffect, useState } from "react";
import { usePlatform } from "../platform/Platform";
import { subscribe } from "./useEvents";

/**
 * Whether this instance is read-only for the open library (its version does
 * not match, its upgrade is incomplete, or no library write lock is held). Same name as the Rust accessor, the Tauri
 * command, and the `library:mode-changed` event payload — one concept, one
 * name at every layer.
 *
 * Write controls remain disabled until mode is known. A newer mode event wins
 * over an older command response; backend enforcement remains authoritative.
 */
export function useLibraryReadOnly(): boolean {
  const platform = usePlatform();
  const [isLibraryReadOnly, setLibraryReadOnly] = useState(true);

  useEffect(() => {
    let cancelled = false;

    let announced = false;
    platform
      .isLibraryReadOnly()
      .then((value) => {
        if (!cancelled && !announced) setLibraryReadOnly(value);
      })
      .catch(() => {
        // Unknown mode must not enable writes. Backend errors have their own sink.
      });
    const unsubscribe = subscribe(() =>
      platform.onLibraryModeChanged((value) => {
        announced = true;
        if (!cancelled) setLibraryReadOnly(value);
      })
    );

    return () => {
      cancelled = true;
      unsubscribe();
    };
  }, [platform]);

  return isLibraryReadOnly;
}
