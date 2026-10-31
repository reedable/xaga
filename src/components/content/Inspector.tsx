import { Tab, Typography } from "@mui/material";
import { TabList, TabContext, TabPanel } from "@mui/lab";
import { useEffect, useState } from "react";
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
  const platform = usePlatform();
  const { pushError } = useErrorSink();
  const [lookup, setLookup] = useState<{
    targetKey: string;
    item: Entry | Annotation | null;
  } | null>(null);
  useEffect(() => {
    const controller = new AbortController();
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
  }, [entryId, annotationId, targetKey, platform, pushError]);
  return (
    <div className={css.Inspector}>
      {showHeading && (
        <Typography className={css.Target} component="h2" color="text.secondary" variant="overline">
          {label} Inspector
        </Typography>
      )}

      <TabContext value={view}>
        <TabList
          onChange={(_, value: InspectorView) => onViewChange(value)}
          aria-label={`${label} Inspector views`}
          variant="fullWidth"
          selectionFollowsFocus
          sx={(theme) => ({
            "& .MuiTabs-indicator": {
              transition: "none",
              "@media (prefers-reduced-motion: no-preference)": {
                transition: theme.transitions.create(),
              },
            },
          })}
        >
          <Tab disableRipple value="related" label="Related" />
          <Tab disableRipple value="topics" label="Topics" />

          {/* TODO Implement me!
          <Tab
            disableRipple
            value="tags"
            label="Tags"
            disabled
          />
          */}
        </TabList>

        <TabPanel className={css.View} value="related" tabIndex={0} keepMounted>
          <RelatedItemList key={targetKey} itemId={itemId} />
        </TabPanel>

        <TabPanel className={css.View} value="topics" tabIndex={0} keepMounted>
          {lookup?.targetKey === targetKey ? (
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
        </TabPanel>
      </TabContext>
    </div>
  );
}
