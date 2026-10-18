import { Typography } from "@mui/material";
import { useCallback, useEffect, useRef, useState } from "react";
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
  // (if any) whose downloaded content was rejected.
  const { pdfBlobUrl, blockedUrl, resolve } =
    useContentLoader(abortControllerRef);

  // Resolve each selected URL before mounting the HTML reader: even a saved
  // feed entry may need ingestion, or may have a persisted PDF representation.
  // Route classification changes must not restart that same resolution.
  const resolveRef = useRef(resolve);
  useEffect(() => {
    resolveRef.current = resolve;
  }, [resolve]);
  const [resolvedUrl, setResolvedUrl] = useState<string | null>(null);
  useEffect(() => {
    abort();
    if (!url) return;
    const signal = abortControllerRef.current.signal;
    void resolveRef.current(url).finally(() => {
      if (!signal.aborted) setResolvedUrl(url);
    });
    return abort;
  }, [url, abort, abortControllerRef]);

  const [readerRevision, setReaderRevision] = useState(0);
  const articleReaderRef = useRef<ArticleReaderHandle>(null);

  const handleReload = useCallback(async () => {
    logger.debug("handleReload", entryId, url, articleReaderRef.current);
    if (url) {
      abort();
      await resolve(url, true);
      setReaderRevision((value) => value + 1);
    } else articleReaderRef.current?.reload();
  }, [entryId, url, abort, resolve]);

  // The current URL's downloaded content was rejected.
  const blocked = !!url && blockedUrl === url;
  const isPdf = mimeType === "application/pdf" && !!url;
  // Wait for local lookup or ingestion before mounting a content reader.
  const isResolving = !!url && resolvedUrl !== url;

  return (
    <div className={cx(className, css.LayoutContainer)}>
      <div className={css.Layout}>
        <Toolbar reload={entryId || url ? handleReload : null} />

        {isResolving ? (
          <div className={css.Placeholder}>Loading…</div>
        ) : blocked ? (
          <div className={css.Placeholder}>
            <Typography variant="body2" color="text.disabled">
              This download can’t be displayed in the reading pane because its
              format isn’t supported.
            </Typography>
          </div>
        ) : isPdf ? (
          <PdfReader src={pdfBlobUrl} />
        ) : entryId ? (
          <ArticleReader key={readerRevision} ref={articleReaderRef} />
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
