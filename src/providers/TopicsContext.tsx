import { createContext, type Dispatch, type SetStateAction } from "react";
import type { Topic } from "../types";

export type TopicsContextType = {
  topics: Topic[];
  /** Derived from `topics` (those with `isPinned`); the content files own pinned state. */
  pinnedTopicIds: string[];
  setTopics: Dispatch<SetStateAction<Topic[]>>;
  setTopicPinned: (id: string, pinned: boolean) => void;
};

const TopicsContext = createContext<TopicsContextType | null>(null);

export default TopicsContext;
