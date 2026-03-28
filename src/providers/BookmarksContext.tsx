import { createContext, type Dispatch, type SetStateAction } from "react";
import type { Entry } from "../types";

export type BookmarkMap = Map<string, Entry>;

export type BookmarksContextType = {
  error: unknown;
  loading: boolean;
  bookmarkMap: BookmarkMap;
  setError: Dispatch<SetStateAction<unknown>>;
  setLoading: Dispatch<SetStateAction<boolean>>;
  setBookmarkMap: Dispatch<SetStateAction<BookmarkMap>>;
};

const BookmarksContext = createContext<BookmarksContextType | null>(null);

export default BookmarksContext;
