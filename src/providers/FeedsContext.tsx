import { createContext, type Dispatch, type SetStateAction } from "react";

export type FeedsContextType = {
  error: unknown;
  loading: boolean;
  loadingSet: Set<string>;
  setError: Dispatch<SetStateAction<unknown>>;
  setLoading: Dispatch<SetStateAction<boolean>>;
  setLoadingSet: Dispatch<SetStateAction<Set<string>>>;
};

const FeedsContext = createContext<FeedsContextType | null>(null);

export default FeedsContext;
