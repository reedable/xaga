import { Tab, Tabs, Typography } from "@mui/material";
import { useEffect, useId, useState } from "react";
import { usePlatform } from "../../platform/Platform";
import { useErrorSink } from "../../hooks/useErrorSink";
import type { Entry } from "../../types";
import { RelatedItemList } from "./RelatedItemList";
import { ArticleTopicMembership } from "./TopicMenuButton";
import css from "./ArticleInspector.module.css";

export type InspectorView = "related" | "topics";

export function ArticleInspector({
  articleId,
  view,
  onViewChange,
}: {
  articleId: string;
  view: InspectorView;
  onViewChange: (view: InspectorView) => void;
}) {
  const id = useId();
  const platform = usePlatform();
  const { pushError } = useErrorSink();
  const [lookup, setLookup] = useState<{
    articleId: string;
    entry: Entry | null;
  } | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    if (view !== "topics") return;
    void platform
      .readEntry(controller.signal, articleId)
      .then((entry) => {
        if (!controller.signal.aborted) setLookup({ articleId, entry });
      })
      .catch((error) => {
        if (!controller.signal.aborted) {
          setLookup({ articleId, entry: null });
          pushError(error);
        }
      });
    return () => controller.abort();
  }, [articleId, view, platform, pushError]);
  return (
    <div className={css.Inspector}>
      <Tabs
        value={view}
        onChange={(_, value: InspectorView) => onViewChange(value)}
        aria-label="Article Inspector views"
        variant="fullWidth"
        selectionFollowsFocus
      >
        <Tab
          disableRipple
          value="related"
          label="Related"
          id={`${id}-related`}
          aria-controls={`${id}-panel-related`}
        />
        <Tab
          disableRipple
          value="topics"
          label="Topics"
          id={`${id}-topics`}
          aria-controls={`${id}-panel-topics`}
        />
        <Tab
          disableRipple
          value="tags"
          label="Tags"
          disabled
          aria-describedby={`${id}-tags-unavailable`}
        />
      </Tabs>
      <Typography
        id={`${id}-tags-unavailable`}
        className={css.Notice}
        variant="caption"
        color="text.secondary"
      >
        Tag assignment isn’t available yet.
      </Typography>
      <div
        className={css.View}
        role="tabpanel"
        id={`${id}-panel-related`}
        aria-labelledby={`${id}-related`}
        tabIndex={0}
        hidden={view !== "related"}
      >
        {view === "related" && <RelatedItemList itemId={articleId} />}
      </div>
      <div
        className={css.View}
        role="tabpanel"
        id={`${id}-panel-topics`}
        aria-labelledby={`${id}-topics`}
        tabIndex={0}
        hidden={view !== "topics"}
      >
        {view !== "topics" ? null : lookup?.articleId === articleId ? (
          lookup.entry ? (
            <ArticleTopicMembership article={lookup.entry} />
          ) : (
            <Typography variant="body2">
              Article topics couldn’t be loaded.
            </Typography>
          )
        ) : (
          <Typography role="status" variant="body2">
            Loading article topics…
          </Typography>
        )}
      </div>
    </div>
  );
}
