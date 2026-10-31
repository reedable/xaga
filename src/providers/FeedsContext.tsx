import { createContext, type Dispatch, type SetStateAction } from "react";
import type { Feed } from "../types";

export type FeedsContextType = {
  error: unknown;
  loading: boolean;
  loadingFeedIds: Set<string>;
  feeds: Feed[];
  hasLoaded?: boolean;
  /** Derived from `feeds` (those with `isPinned`); the shared library pin store owns membership. */
  pinnedFeedIds: Feed["id"][];
  setError: Dispatch<SetStateAction<unknown>>;
  setLoading: Dispatch<SetStateAction<boolean>>;
  setLoadingSet: Dispatch<SetStateAction<Set<string>>>;
  setFeeds: Dispatch<SetStateAction<Feed[]>>;
  setFeedPinned: (id: Feed["id"], pinned: boolean) => void;
};

const FeedsContext = createContext<FeedsContextType | null>(null);

export default FeedsContext;
