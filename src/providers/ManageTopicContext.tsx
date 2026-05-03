import { createContext } from "react";
import type { Topic } from "../types";

export type ManageTopicContextType = {
  topics: Topic[];
  pinnedTopicIds: string[];
  startEditingTopic: (topic: Topic) => void;
  startAddingTopic: () => void;
};

const ManageTopicContext = createContext<ManageTopicContextType | null>(null);

export default ManageTopicContext;
