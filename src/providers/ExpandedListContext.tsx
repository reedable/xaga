import { createContext } from "react";

/** Which top-level sidebar section is expanded (single-open accordion). */
export type ListType = "SHORTCUTS" | "TOPICS" | "FEEDS" | "SITES" | "TAGS";

export type ExpandedListContextType = {
  expandedList: ListType | null;
  setExpandedList: (
    updater: ((prev: ListType | null) => ListType | null) | (ListType | null)
  ) => void;
};

const ExpandedListContext = createContext<ExpandedListContextType | null>(null);

export default ExpandedListContext;
