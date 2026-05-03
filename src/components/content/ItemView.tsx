import { Typography } from "@mui/material";
import { useEffect, useRef, useState } from "react";
import { useArticles } from "../../hooks/useArticles";
import { useEvents } from "../../hooks/useEvents";
import { type Entry } from "../../types";
import cx from "../../utils/cx";
import ArticleReader from "./ArticleReader";
import css from "./_.module.css";

interface ItemViewProps {
  className?: string;
  itemId: string | null;
}

export function ItemView({ className, itemId }: ItemViewProps) {
  const abortControllerRef = useRef(new AbortController());
  const { readArticle } = useArticles(abortControllerRef);
  const [entry, setEntry] = useState<Entry | null>(null);

  const { onUnreadChanged, onBookmarkChanged } = useEvents();

  useEffect(() => {
    if (!itemId) { setEntry(null); return; }
    readArticle(itemId).then((result) => setEntry(result ?? null));
  }, [itemId, readArticle]);

  useEffect(() => {
    return onUnreadChanged((updated) => {
      setEntry((prev) => (prev?.id === updated.id ? updated : prev));
    });
  }, [onUnreadChanged]);

  useEffect(() => {
    return onBookmarkChanged((updated) => {
      setEntry((prev) => (prev?.id === updated.id ? updated : prev));
    });
  }, [onBookmarkChanged]);

  if (!itemId) {
    return (
      <div className={cx(className, css.Placeholder)}>
        <Typography variant="body2" color="text.disabled">
          Select an article
        </Typography>
      </div>
    );
  }

  return (
    <div className={cx(className, css.LayoutContainer)}>
      <div className={css.Layout}>
        {entry && <ArticleReader entryId={entry.id} />}
      </div>
    </div>
  );
}
