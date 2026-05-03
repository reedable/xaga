import { createContext } from "react";
import type { Feed } from "../types";

export type ManageFeedContextType = {
  feeds: Feed[];
  pinnedFeedIds: string[];
  loadingFeedIds: Set<string>;
  startEditingFeed: (feed: Feed) => void;
  startAddingFeed: () => void;
};

const ManageFeedContext = createContext<ManageFeedContextType | null>(null);

export default ManageFeedContext;
