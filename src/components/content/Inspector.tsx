import { Tab, Tabs, Typography } from "@mui/material";
import { useEffect, useId, useState } from "react";
import { usePlatform } from "../../platform/Platform";
import { useErrorSink } from "../../hooks/useErrorSink";
import type { Annotation, Entry } from "../../types";
import { RelatedItemList } from "./RelatedItemList";
import { TopicMembership } from "./TopicMembership";
import css from "./Inspector.module.css";

export type InspectorView = "related" | "topics";

export type InspectorTarget = { entryId: string; annotationId?: string };

export function Inspector({
  target,
  view,
  onViewChange,
  showHeading = true,
}: {
  target: InspectorTarget;
  view: InspectorView;
  onViewChange: (view: InspectorView) => void;
  showHeading?: boolean;
}) {
  const { entryId, annotationId } = target;
  const itemId = annotationId ?? entryId;
  const targetKey = JSON.stringify([entryId, annotationId]);
  const label = annotationId ? "Annotation" : "Article";
  const id = useId();
  const platform = usePlatform();
  const { pushError } = useErrorSink();
  const [lookup, setLookup] = useState<{
    targetKey: string;
    item: Entry | Annotation | null;
  } | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    if (view !== "topics") return;
    const read = annotationId
      ? platform
          .readArticleScope(controller.signal, entryId)
          .then(
            (article) =>
              article.annotations.find(
                (annotation) => annotation.id === annotationId
              ) ?? null
          )
      : platform.readEntry(controller.signal, entryId);
    void read
      .then((item) => {
        if (!controller.signal.aborted) setLookup({ targetKey, item });
      })
      .catch((error) => {
        if (!controller.signal.aborted) {
          setLookup({ targetKey, item: null });
          pushError(error);
        }
      });
    return () => controller.abort();
  }, [entryId, annotationId, targetKey, view, platform, pushError]);
  return (
    <div className={css.Inspector}>
      {showHeading && (
        <div className={css.Target}>
          <Typography variant="subtitle2">{label} Inspector</Typography>
        </div>
      )}
      <Tabs
        value={view}
        onChange={(_, value: InspectorView) => onViewChange(value)}
        aria-label={`${label} Inspector views`}
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
        {view === "related" && (
          <RelatedItemList key={targetKey} itemId={itemId} />
        )}
      </div>
      <div
        className={css.View}
        role="tabpanel"
        id={`${id}-panel-topics`}
        aria-labelledby={`${id}-topics`}
        tabIndex={0}
        hidden={view !== "topics"}
      >
        {view !== "topics" ? null : lookup?.targetKey === targetKey ? (
          lookup.item ? (
            <TopicMembership key={targetKey} item={lookup.item} />
          ) : (
            <Typography variant="body2">
              {label} topics couldn’t be loaded.
            </Typography>
          )
        ) : (
          <Typography role="status" variant="body2">
            Loading {label.toLowerCase()} topics…
          </Typography>
        )}
      </div>
    </div>
  );
}
