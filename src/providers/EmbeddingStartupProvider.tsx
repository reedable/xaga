import { useEffect, useMemo, useState, type ReactNode } from "react";
import type { EmbeddingStartupState } from "../platform/Platform";
import { usePlatform } from "../platform/Platform";
import EmbeddingStartupContext from "./EmbeddingStartupContext";

export function EmbeddingStartupProvider({
  children,
}: {
  children: ReactNode;
}) {
  const platform = usePlatform();
  const [state, setState] = useState<EmbeddingStartupState>("loading");

  useEffect(() => {
    let cancelled = false;

    const poll = async () => {
      while (!cancelled) {
        try {
          const next = await platform.embeddingStartupState();
          if (cancelled) return;
          setState(next);
          if (next !== "loading") return;
        } catch {
          if (!cancelled) setState("unavailable");
          return;
        }
        await new Promise((resolve) => setTimeout(resolve, 100));
      }
    };

    void poll();
    return () => {
      cancelled = true;
    };
  }, [platform]);

  const value = useMemo(() => state, [state]);
  return (
    <EmbeddingStartupContext.Provider value={value}>
      {children}
    </EmbeddingStartupContext.Provider>
  );
}
