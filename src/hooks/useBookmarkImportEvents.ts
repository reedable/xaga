import { useEffect } from "react";
import { usePlatform } from "../platform/Platform";
import { importFailure } from "../utils/importFailure";
import { useErrorSink } from "./useErrorSink";
import { subscribe } from "./useEvents";

export function useBookmarkImportEvents() {
  const platform = usePlatform();
  const { pushError } = useErrorSink();
  useEffect(
    () =>
      subscribe(() =>
        platform.onBookmarksImported((outcome) => {
          const failure = importFailure("bookmarks", outcome);
          if (failure) pushError(failure);
        })
      ),
    [platform, pushError]
  );
  useEffect(
    () =>
      subscribe(() =>
        platform.onBookmarksExported((outcome) => {
          if (outcome.error) pushError(outcome.error);
        })
      ),
    [platform, pushError]
  );
}
