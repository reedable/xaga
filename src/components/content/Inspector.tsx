import { Tab, Typography } from "@mui/material";
import { TabList, TabContext, TabPanel } from "@mui/lab";
import { useEffect, useState } from "react";
import { usePlatform } from "../../platform/Platform";
import { useErrorSink } from "../../hooks/useErrorSink";
import type { Annotation, Entry } from "../../types";
import { RelatedEntityList } from "./RelatedEntityList";
import { TopicMembership } from "./TopicMembership";
import { EntityTags } from "./EntityTags";
import css from "./Inspector.module.css";

export type InspectorView = "related" | "topics" | "tags";

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
  const entityId = annotationId ?? entryId;
  const targetKey = JSON.stringify([entryId, annotationId]);
  const label = annotationId !== undefined ? "Annotation" : "Article";
  const platform = usePlatform();
  const { pushError } = useErrorSink();

  const [lookup, setLookup] = useState<{
    targetKey: string;
    entity: Entry | Annotation | null;
  } | null>(null);

  useEffect(() => {
    const controller = new AbortController();

    const read =
      annotationId !== undefined
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
      .then((entity) => {
        if (!controller.signal.aborted) setLookup({ targetKey, entity });
      })
      .catch((error) => {
        if (!controller.signal.aborted) {
          setLookup({ targetKey, entity: null });
          pushError(error);
        }
      });
    return () => controller.abort();
  }, [entryId, annotationId, targetKey, platform, pushError]);

  return (
    <div className={css.Inspector}>
      {showHeading && (
        <Typography
          className={css.Target}
          component="h2"
          color="text.secondary"
          variant="overline"
        >
          {label} details
        </Typography>
      )}

      <TabContext value={view}>
        <TabList
          onChange={(_, value: InspectorView) => onViewChange(value)}
          aria-label={`${label} details views`}
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
          <Tab
            data-testid="inspector-related"
            disableRipple
            value="related"
            label="Related"
          />
          <Tab
            data-testid="inspector-topics"
            disableRipple
            value="topics"
            label="Topics"
          />

          <Tab
            data-testid="inspector-tags"
            disableRipple
            value="tags"
            label="Tags"
          />
        </TabList>

        <TabPanel className={css.View} value="tags" tabIndex={0} keepMounted>
          <EntityTags
            key={targetKey}
            entryId={entryId}
            annotationId={annotationId}
          />
        </TabPanel>

        <TabPanel className={css.View} value="related" tabIndex={0} keepMounted>
          <RelatedEntityList key={targetKey} entityId={entityId} />
        </TabPanel>

        <TabPanel className={css.View} value="topics" tabIndex={0} keepMounted>
          {lookup?.targetKey === targetKey ? (
            lookup.entity ? (
              <TopicMembership key={targetKey} entity={lookup.entity} />
            ) : (
              <Typography role="alert" variant="body2">
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
