import { useMemo, useState, type ReactNode } from "react";
import FeedsContext, { type FeedsContextType } from "./FeedsContext";

export default function FeedsProvider({ children }: { children: ReactNode }) {
  const [error, setError] = useState<unknown>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [loadingSet, setLoadingSet] = useState<Set<string>>(new Set());

  const value = useMemo<FeedsContextType>(
    () => ({
      error,
      loading,
      loadingSet,
      setError,
      setLoading,
      setLoadingSet,
    }),
    [error, loading, loadingSet],
  );

  return (
    <FeedsContext.Provider value={value}>{children}</FeedsContext.Provider>
  );
}
