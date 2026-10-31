import { createContext, type Dispatch, type SetStateAction } from "react";
import type { Topic } from "../types";

export type TopicsContextType = {
  topics: Topic[];
  hasLoaded?: boolean;
  /** Derived from `topics` (those with `isPinned`); the shared library pin store owns membership. */
  pinnedTopicIds: string[];
  setTopics: Dispatch<SetStateAction<Topic[]>>;
  setTopicPinned: (id: string, pinned: boolean) => void;
  setMembership: (topicId: string, itemId: string, present: boolean) => void;
  getMemberships: (itemId: string, indexedIds: string[]) => string[];
};

const TopicsContext = createContext<TopicsContextType | null>(null);

export default TopicsContext;
