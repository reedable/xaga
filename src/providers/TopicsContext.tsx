import {
  createContext,
  type Dispatch,
  type SetStateAction,
} from "react";
import type { Topic } from "../types";

export type TopicsContextType = {
  topics: Topic[];
  pinnedTopicIds: string[];
  setTopics: Dispatch<SetStateAction<Topic[]>>;
  setPinnedTopicIds: Dispatch<SetStateAction<string[]>>;
};

const TopicsContext = createContext<TopicsContextType | null>(null);

export default TopicsContext;
