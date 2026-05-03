import { useEffect, useState } from "react";
import { usePlatform } from "../platform/Platform";

export function useCacheReady(): boolean {
  const platform = usePlatform();
  const [ready, setReady] = useState(false);

  useEffect(() => {
    let cancelled = false;

    const poll = async () => {
      while (!cancelled) {
        if (await platform.isCacheSeeded()) {
          if (!cancelled) setReady(true);
          return;
        }
        await new Promise(r => setTimeout(r, 100));
      }
    };

    poll();
    return () => { cancelled = true; };
  }, [platform]);

  return ready;
}
