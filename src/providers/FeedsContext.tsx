import { createContext, type Dispatch, type SetStateAction } from "react";
import type { Feed } from "../types";

export type FeedsContextType = {
  error: unknown;
  loading: boolean;
  loadingFeedIds: Set<string>;
  feeds: Feed[];
  pinnedFeedIds: Feed["id"][];
  setError: Dispatch<SetStateAction<unknown>>;
  setLoading: Dispatch<SetStateAction<boolean>>;
  setLoadingSet: Dispatch<SetStateAction<Set<string>>>;
  setFeeds: Dispatch<SetStateAction<Feed[]>>;
  setPinnedFeedIds: Dispatch<SetStateAction<Feed["id"][]>>;
};

const FeedsContext = createContext<FeedsContextType | null>(null);

export default FeedsContext;
