import { createContext, useContext } from "react";
import type { Tag, TagMerge } from "../domain";

type TagsContextValue = {
  tags: Tag[];
  hasLoaded: boolean;
  renaming?: string[];
  deleting?: string[];
  merging?: TagMerge[];
  unreadCount: number | null;
};

export const TagsContext = createContext<TagsContextValue | null>(null);
export function useTags() {
  const context = useContext(TagsContext);
  if (!context) throw new Error("useTags must be used inside TagsProvider");
  return context;
}
