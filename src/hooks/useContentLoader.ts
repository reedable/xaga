import { useCallback, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import useRouter from "./useRouter";
import { useArticles } from "./useArticles";
import { useSettings } from "./useSettings";

export function useContentLoader(abortControllerRef: RefObject<AbortController>) {
  const platform = usePlatform();
  const { navigate, state, searchParams } = useRouter();
  const { findArticleByUrl, loadArticleByUrl, loading } = useArticles(abortControllerRef);
  const { inlinePdfReader } = useSettings();

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
        if (inlinePdfReader) {
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
    [platform, navigate, state, searchParams, findArticleByUrl, loadArticleByUrl, inlinePdfReader]
  );

  return { loadUrl, loading };
}
