import { Typography } from "@mui/material";
import { useCallback, useEffect, useRef } from "react";
import useAbortController from "../../hooks/useAbortController";
import { useContentLoader } from "../../hooks/useContentLoader";
import useRouter from "../../hooks/useRouter";
import cx from "../../utils/cx";
import Logger from "../../utils/Logger";
import ArticleReader, { type ArticleReaderHandle } from "./ArticleReader";
import css from "./ItemView.module.css";
import { PdfReader } from "./PdfReader";
import { Toolbar } from "./Toolbar";

type ItemViewProps = {
  className?: string;
};

const logger = Logger.getLogger("ItemView");

export function ItemView({ className }: ItemViewProps) {
  const { state } = useRouter();
  const { entryId, url, mimeType } = state;

  const [abort, abortControllerRef] = useAbortController();
  // Resolves a bare URL (link / URL bar) into a presentation and writes the
  // result back onto the route. Returns the inline-PDF blob URL, and the URL
  // (if any) whose probe was blocked as a spoofed PDF.
  const { pdfBlobUrl, blockedUrl, resolve } =
    useContentLoader(abortControllerRef);

  // Only a bare, unclassified URL needs resolving; entryId means a saved item is
  // selected, and a set mimeType means it's already resolved.
  useEffect(() => {
    if (!url || entryId || mimeType) return;
    abort();
    resolve(url);
    return abort;
  }, [url, entryId, mimeType, abort, resolve]);

  const articleReaderRef = useRef<ArticleReaderHandle>(null);

  const handleReload = useCallback(() => {
    logger.debug("handleReload", entryId, url, articleReaderRef.current);
    articleReaderRef.current?.reload();
  }, [entryId, url]);

  // The current URL's inline-PDF probe was blocked as a spoof.
  const blocked = !!url && blockedUrl === url;
  const isPdf = mimeType === "application/pdf" && !!url;
  // A bare URL the content loader is still classifying (no entryId, no mimeType).
  const isResolving = !!url && !entryId && !mimeType;

  return (
    <div className={cx(className, css.LayoutContainer)}>
      <div className={css.Layout}>
        <Toolbar reload={entryId || url ? handleReload : null} />

        {blocked ? (
          <div className={css.Placeholder}>
            <Typography variant="body2" color="text.disabled">
              Blocked: this link claimed to be a PDF but the content is not a PDF.
            </Typography>
          </div>
        ) : isPdf ? (
          <PdfReader src={pdfBlobUrl} />
        ) : entryId ? (
          <ArticleReader ref={articleReaderRef} />
        ) : (
          <div className={css.Placeholder}>
            <Typography variant="body2" color="text.disabled">
              {isResolving ? "Loading…" : "Select an article"}
            </Typography>
          </div>
        )}
      </div>
    </div>
  );
}
