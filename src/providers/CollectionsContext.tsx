import { createContext, type Dispatch, type SetStateAction } from "react";
import type { Collection } from "../types";

export type CollectionsContextType = {
  collections: Collection[];
  pinnedCollectionIds: string[];
  setCollections: Dispatch<SetStateAction<Collection[]>>;
  setPinnedCollectionIds: Dispatch<SetStateAction<string[]>>;
};

const CollectionsContext = createContext<CollectionsContextType | null>(null);

export default CollectionsContext;
