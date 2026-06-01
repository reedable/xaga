import { useCallback, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import useRouter from "./useRouter";
import { useArticles } from "./useArticles";
import { usePreference } from "./usePreference";

export function useContentLoader(abortControllerRef: RefObject<AbortController>) {
  const platform = usePlatform();
  const { navigate, state, searchParams } = useRouter();
  const { findArticleByUrl, loadArticleByUrl, loading } = useArticles(abortControllerRef);
  const [inlinePdfViewer] = usePreference("inlinePdfViewer", false);

  const loadUrl = useCallback(
    async (url: string) => {
      const params = new URLSearchParams(searchParams);

      const existing = await findArticleByUrl(url);
      if (existing) {
        navigate(`/?${params.toString()}`, {
          state: { ...state, entryId: existing.id, url, mimeType: "text/html" },
        });
        return;
      }

      const result = await loadArticleByUrl(url);
      if (!result) return;

      const { article, mimeType } = result;

      if (mimeType === "application/pdf") {
        if (inlinePdfViewer) {
          navigate(`/?${params.toString()}`, {
            state: { ...state, url, mimeType, entryId: undefined, annotationId: undefined },
          });
        } else {
          platform.openUrl(url);
        }
        return;
      }

      if (!article) return;

      navigate(`/?${params.toString()}`, {
        state: { ...state, entryId: article.id, url, mimeType },
      });
    },
    [platform, navigate, state, searchParams, findArticleByUrl, loadArticleByUrl, inlinePdfViewer]
  );

  return { loadUrl, loading };
}
