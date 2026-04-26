import { useCallback, useState } from "react";

export function useLocalStorage<T>(
  key: string,
  defaultValue: T,
): [T, React.Dispatch<React.SetStateAction<T>>] {
  const [value, setValue] = useState<T>(() => {
    try {
      const stored = localStorage.getItem(key);
      return stored !== null ? (JSON.parse(stored) as T) : defaultValue;
    } catch {
      return defaultValue;
    }
  });

  const setAndPersist: React.Dispatch<React.SetStateAction<T>> = useCallback(
    (action) => {
      setValue((prev) => {
        const next =
          typeof action === "function"
            ? (action as (prev: T) => T)(prev)
            : action;
        try {
          localStorage.setItem(key, JSON.stringify(next));
        } catch {
          // storage quota exceeded or private browsing
        }
        return next;
      });
    },
    [key],
  );

  return [value, setAndPersist];
}
