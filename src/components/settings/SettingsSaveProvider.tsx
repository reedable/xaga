import { useEffect, useRef, type ReactNode } from "react";
import { usePlatform } from "../../platform/Platform";
import { useErrorSink } from "../../hooks/useErrorSink";
import { subscribe } from "../../hooks/useEvents";

import { SettingsSaveContext } from "./SettingsSaveContext";

export function SettingsSaveProvider({ children }: { children: ReactNode }) {
  const platform = usePlatform();
  const { pushError } = useErrorSink();
  const saves = useRef(new Set<() => Promise<boolean>>());
  const closing = useRef<Promise<boolean> | null>(null);
  async function saveAll() {
    let allowed = true;
    for (const save of saves.current) {
      if (!(await save())) allowed = false;
    }
    return allowed;
  }
  useEffect(
    () =>
      subscribe(() =>
        platform
          .onWindowCloseRequested(() => {
            closing.current ??= saveAll()
              .catch((error) => {
                pushError(error);
                return false;
              })
              .finally(() => {
                closing.current = null;
              });
            return closing.current;
          })
          .catch((error) => {
            pushError(error);
            return () => {};
          })
      ),
    [platform, pushError]
  );
  return (
    <SettingsSaveContext.Provider value={saves.current}>
      {children}
    </SettingsSaveContext.Provider>
  );
}
