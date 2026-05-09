import { Collapse, LinearProgress } from "@mui/material";
import { useCallback, useEffect, useRef, useState } from "react";
import useAbortController from "../hooks/useAbortController";
import { useArticles } from "../hooks/useArticles";
import useRouter from "../hooks/useRouter";
import css from "./AddressBar.module.css";

interface AddressBarProps {
  url: string | null;
}

export function AddressBar({ url }: AddressBarProps) {
  const { navigate, state, searchParams } = useRouter();
  const [, abortControllerRef] = useAbortController();
  const { loading, findArticleByUrl, loadArticleByUrl } =
    useArticles(abortControllerRef);

  const [addressValue, setAddressValue] = useState(url ?? "");
  const [isEditing, setIsEditing] = useState(false);
  const isNavigatingRef = useRef(false);

  useEffect(() => {
    if (!isEditing) {
      (async () => setAddressValue(url ?? ""))();
    }
  }, [url, isEditing]);

  const navigateToUrl = useCallback(
    async (inputUrl: string) => {
      if (!/^https?:\/\//i.test(inputUrl)) inputUrl = `https://${inputUrl}`;
      let article = await findArticleByUrl(inputUrl);
      if (!article) article = await loadArticleByUrl(inputUrl);
      if (!article) return;
      const params = new URLSearchParams(searchParams);
      navigate(`/?${params.toString()}`, {
        state: { ...state, entryId: article.id },
      });
    },
    [findArticleByUrl, loadArticleByUrl, navigate, state, searchParams]
  );

  const handleFocus = useCallback((e: React.FocusEvent<HTMLInputElement>) => {
    setIsEditing(true);
    const target = e.target;
    requestAnimationFrame(() => target.select());
  }, []);

  const handleBlur = useCallback(() => {
    setIsEditing(false);
    if (!isNavigatingRef.current) setAddressValue(url ?? "");
    isNavigatingRef.current = false;
  }, [url]);

  const handleChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value.replace(/^http:\/\/(?!\/)/i, "https://");
    setAddressValue(value);
  }, []);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "Enter") {
        const value = addressValue.trim();
        if (value) {
          isNavigatingRef.current = true;
          void navigateToUrl(value);
        }
        e.currentTarget.blur();
      } else if (e.key === "Escape") {
        e.currentTarget.blur();
      }
    },
    [addressValue, navigateToUrl]
  );

  return (
    <>
      <div className={css.AddressBar}>
        <input
          className={css.AddressBarInput}
          value={addressValue}
          placeholder="Enter URL…"
          onChange={handleChange}
          onFocus={handleFocus}
          onBlur={handleBlur}
          onKeyDown={handleKeyDown}
          autoComplete="off"
          autoCorrect="off"
          autoCapitalize="off"
          spellCheck={false}
        />
      </div>
      <Collapse in={loading} unmountOnExit>
        <LinearProgress />
      </Collapse>
    </>
  );
}
