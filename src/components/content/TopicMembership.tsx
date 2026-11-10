import { Close } from "@mui/icons-material";
import {
  Alert,
  Box,
  Button,
  Checkbox,
  IconButton,
  Tooltip,
  InputAdornment,
  LinearProgress,
  List,
  ListItem,
  ListItemButton,
  ListItemText,
  MenuItem,
  Select,
  type SelectChangeEvent,
  TextField,
  Typography,
  useMediaQuery,
} from "@mui/material";
import { motion } from "framer-motion";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import useAbortController from "../../hooks/useAbortController";
import { useLibraryReadOnly } from "../../hooks/useLibraryReadOnly";
import { useTopics } from "../../hooks/useTopics";
import { useEmbeddingStartupState } from "../../hooks/useEmbeddingStartupState";
import type { Annotation, Entry } from "../../types";
import { TopicFormFields, useTopicForm } from "../dialogs/TopicFormFields";
import css from "./TopicMembership.module.css";

const MotionListItem = motion(ListItem);

function InlineTopicForm({
  initialTitle,
  onCancel,
  onCreate,
  readOnly,
}: {
  initialTitle: string;
  readOnly: boolean;
  onCancel: () => void;
  onCreate: (
    values: ReturnType<typeof useTopicForm>["values"]
  ) => Promise<boolean>;
}) {
  const form = useTopicForm({ title: initialTitle });
  const [saving, setSaving] = useState(false);

  const handleSubmit = async (event: React.SubmitEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!form.values.title || readOnly) return;
    setSaving(true);
    try {
      if (await onCreate(form.values)) onCancel();
    } finally {
      setSaving(false);
    }
  };

  return (
    <Box component="form" onSubmit={handleSubmit} className={css.InlineForm}>
      <TopicFormFields form={form} busy={saving || readOnly} autoFocus />
      <div className={css.InlineFormActions}>
        <Button data-testid="cancel" onClick={onCancel} disabled={saving}>
          Cancel
        </Button>
        <Button
          data-testid="confirm-create-topic"
          type="submit"
          variant="contained"
          disabled={!form.values.title || saving || readOnly}
        >
          Create Topic
        </Button>
      </div>
    </Box>
  );
}

export function TopicMembership({ entity }: { entity: Entry | Annotation }) {
  const [, abortControllerRef] = useAbortController();
  const readOnly = useLibraryReadOnly();
  const animate = useMediaQuery("(prefers-reduced-motion: no-preference)");
  const mutationPending = useRef(false);
  const [mutating, setMutating] = useState(false);
  const currentEntityId = useRef(entity.id);
  useEffect(() => {
    currentEntityId.current = entity.id;
  }, [entity.id]);
  const embeddingStartupState = useEmbeddingStartupState();
  const {
    topics,
    hasLoaded,
    createTopicWithMember,
    setTopicPinned,
    listEntryTopics,
    addTopicEntity,
    removeTopicEntity,
    rankTopicsRelatedToEntity,
  } = useTopics(abortControllerRef);

  const [membershipState, setMembershipState] = useState<
    "loading" | "available" | "unavailable"
  >("loading");
  const [membershipRetry, setMembershipRetry] = useState(0);
  const [memberIds, setMemberIds] = useState<Set<string>>(new Set());
  const [filterText, setFilterText] = useState("");
  const [sortBy, setSortBy] = useState<"name" | "relevance">(
    embeddingStartupState === "available" ? "relevance" : "name"
  );
  const [similarityMap, setSimilarityMap] = useState<Map<string, number>>(
    new Map()
  );
  const [relevanceLoading, setRelevanceLoading] = useState(false);
  const [creating, setCreating] = useState(false);
  const [createInitialTitle, setCreateInitialTitle] = useState("");
  const filterInputRef = useRef<HTMLInputElement>(null);

  const entityId = entity.id;

  // Reset on entity change so selection stays cleared until the new entity's
  // memberships load; keyed on the id so entry object identity churn from
  // bookmark/unread events doesn't wipe and refetch the same memberships.
  const [lastEntityId, setLastEntityId] = useState(entityId);
  if (lastEntityId !== entityId) {
    setLastEntityId(entityId);
    setMemberIds(new Set());
    setMembershipState("loading");
  }

  useEffect(() => {
    if (!entityId) return;
    const controller = new AbortController();
    setMembershipState("loading");
    listEntryTopics(entityId).then((ids) => {
      if (controller.signal.aborted) return;
      if (ids === undefined) {
        setMembershipState("unavailable");
      } else {
        setMemberIds(new Set(ids));
        setMembershipState("available");
      }
    });
    return () => controller.abort();
  }, [entityId, listEntryTopics, topics, membershipRetry]);

  useEffect(() => {
    if (
      !entityId ||
      sortBy !== "relevance" ||
      embeddingStartupState !== "available"
    )
      return;
    let cancelled = false;
    setRelevanceLoading(true);
    void rankTopicsRelatedToEntity(entityId).then((results) => {
      if (cancelled) return;
      setSimilarityMap(
        new Map((results ?? []).map((result) => [result.id, result.similarity]))
      );
      setRelevanceLoading(false);
    });
    return () => {
      cancelled = true;
    };
  }, [entityId, sortBy, embeddingStartupState, rankTopicsRelatedToEntity]);

  const startCreating = useCallback(() => {
    setCreateInitialTitle(topics.length > 0 ? filterText.trim() : "");
    setCreating(true);
  }, [filterText, topics.length]);

  const stopCreating = useCallback(() => {
    setCreating(false);
    requestAnimationFrame(() => filterInputRef.current?.focus());
  }, []);

  const handleCreate = useCallback(
    async (values: ReturnType<typeof useTopicForm>["values"]) => {
      if (readOnly) return false;
      const input = {
        title: values.title,
        description: values.description,
        threshold: values.threshold,
      };
      const topic = await createTopicWithMember(input, entity);
      if (!topic) return false;
      setTopicPinned(topic.id, values.pinned);
      if (currentEntityId.current !== entity.id) return true;
      setMemberIds((previous) => new Set([...previous, topic.id]));
      return true;
    },
    [createTopicWithMember, entity, setTopicPinned, readOnly]
  );

  const handleSortByChange = (event: SelectChangeEvent) => {
    setSortBy(event.target.value as "name" | "relevance");
  };

  const handleToggle = useCallback(
    async (topicId: string) => {
      if (readOnly) return;
      if (mutationPending.current) return;
      mutationPending.current = true;
      setMutating(true);
      try {
        const removing = memberIds.has(topicId);
        const topic = removing
          ? await removeTopicEntity(topicId, entity.id)
          : await addTopicEntity(topicId, entity);
        if (!topic || currentEntityId.current !== entity.id) return;
        setMemberIds((previous) => {
          const next = new Set(previous);
          if (removing) next.delete(topicId);
          else next.add(topicId);
          return next;
        });
      } finally {
        mutationPending.current = false;
        setMutating(false);
      }
    },
    [entity, memberIds, addTopicEntity, removeTopicEntity, readOnly]
  );

  const sortedTopics = useMemo(
    () => [...topics].sort((a, b) => a.title.localeCompare(b.title)),
    [topics]
  );

  const filteredTopics = useMemo(() => {
    const base = filterText
      ? sortedTopics.filter((t) =>
          t.title.toLowerCase().includes(filterText.toLowerCase())
        )
      : sortedTopics;
    return [...base].sort((a, b) => {
      const aChecked = memberIds.has(a.id);
      const bChecked = memberIds.has(b.id);
      if (aChecked !== bChecked) return aChecked ? -1 : 1;
      if (sortBy === "relevance") {
        return (similarityMap.get(b.id) ?? 0) - (similarityMap.get(a.id) ?? 0);
      }
      return 0;
    });
  }, [sortedTopics, filterText, memberIds, sortBy, similarityMap]);

  if (!hasLoaded || membershipState === "loading") {
    return (
      <Typography className={css.Filter} role="status" variant="body2">
        Loading Topics…
      </Typography>
    );
  }

  if (membershipState === "unavailable") {
    return (
      <Alert
        severity="warning"
        action={
          <Button
            data-testid="retry-topic-memberships"
            onClick={() => setMembershipRetry((value) => value + 1)}
          >
            Retry
          </Button>
        }
      >
        Topic memberships could not be read. Retry, or reopen Xaga after
        resolving the library problem.
      </Alert>
    );
  }

  return (
    <div className={css.Content}>
      {relevanceLoading && <LinearProgress />}

      <div className={css.Header}>
        <div className={css.HeaderRight}>
          <Select
            aria-label="Sort by"
            className={css.SortSelect}
            value={sortBy}
            size="small"
            variant="standard"
            onChange={handleSortByChange}
          >
            <MenuItem
              value="relevance"
              disabled={embeddingStartupState !== "available"}
            >
              Sort by most relevant
            </MenuItem>

            <MenuItem value="name">Sort alphabetically</MenuItem>
          </Select>
        </div>
      </div>

      {!creating && (
        <div className={css.Filter}>
          <TextField
            data-testid="filter-topics"
            className={css.FilterTextInput}
            placeholder="Filter topics…"
            size="small"
            fullWidth
            value={filterText}
            inputRef={filterInputRef}
            onChange={(e) => setFilterText(e.target.value)}
            autoComplete="off"
            variant="standard"
            slotProps={{
              htmlInput: { spellCheck: false },
              input: {
                endAdornment: filterText && (
                  <InputAdornment position="end">
                    <Tooltip title="Clear">
                      <IconButton
                        size="small"
                        edge="end"
                        onClick={() => setFilterText("")}
                        aria-label="Clear"
                      >
                        <Close fontSize="small" />
                      </IconButton>
                    </Tooltip>
                  </InputAdornment>
                ),
              },
            }}
          />
        </div>
      )}

      {creating ? (
        <InlineTopicForm
          readOnly={readOnly}
          initialTitle={createInitialTitle}
          onCancel={stopCreating}
          onCreate={handleCreate}
        />
      ) : (
        <List className={css.List} disablePadding>
          {filteredTopics.map((t) => (
            <MotionListItem
              key={t.id}
              layout={animate}
              transition={{ duration: animate ? 0.3 : 0, ease: "easeOut" }}
              disablePadding
            >
              <ListItemButton
                onClick={() => handleToggle(t.id)}
                disabled={readOnly || mutating}
                dense
                role="checkbox"
                aria-checked={memberIds.has(t.id)}
              >
                <Checkbox
                  tabIndex={-1}
                  disabled={readOnly || mutating}
                  slotProps={{ input: { "aria-hidden": true } }}
                  checked={memberIds.has(t.id)}
                  size="small"
                  disableRipple
                  className={css.Checkbox}
                />
                <ListItemText primary={t.title} />
              </ListItemButton>
            </MotionListItem>
          ))}
          {(topics.length === 0 ||
            (filterText.trim() && filteredTopics.length === 0)) && (
            <ListItem>
              <Button
                data-testid="create-topic"
                onClick={startCreating}
                disabled={readOnly}
                fullWidth
                variant="outlined"
              >
                Create a New Topic
              </Button>
            </ListItem>
          )}
        </List>
      )}
    </div>
  );
}
