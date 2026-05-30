import { useCallback, useEffect, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import { usePlatform } from "../platform/Platform";

export function usePreference<T>(
  key: string,
  defaultValue: T,
): [T, Dispatch<SetStateAction<T>>] {
  const platform = usePlatform();
  const [value, setRawValue] = useState<T>(defaultValue);

  useEffect(() => {
    platform.getUiPref<T>(key).then((stored) => {
      if (stored !== null) setRawValue(stored);
    });
  }, [platform, key]);

  const setValue = useCallback(
    (updater: SetStateAction<T>) => {
      setRawValue((prev) => {
        const next =
          typeof updater === "function"
            ? (updater as (p: T) => T)(prev)
            : updater;
        platform.setUiPref(key, next);
        return next;
      });
    },
    [platform, key],
  );

  return [value, setValue];
}
