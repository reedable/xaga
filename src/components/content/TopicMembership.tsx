import { Close } from "@mui/icons-material";
import {
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
        <Button onClick={onCancel} disabled={saving}>
          Cancel
        </Button>
        <Button
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

export function TopicMembership({ item }: { item: Entry | Annotation }) {
  const [, abortControllerRef] = useAbortController();
  const readOnly = useLibraryReadOnly();
  const animate = useMediaQuery("(prefers-reduced-motion: no-preference)");
  const mutationPending = useRef(false);
  const [mutating, setMutating] = useState(false);
  const currentItemId = useRef(item.id);
  useEffect(() => {
    currentItemId.current = item.id;
  }, [item.id]);
  const embeddingStartupState = useEmbeddingStartupState();
  const {
    topics,
    hasLoaded,
    createTopicWithMember,
    setTopicPinned,
    listEntryTopics,
    addTopicItem,
    removeTopicItem,
    rankTopicsRelatedToItem,
  } = useTopics(abortControllerRef);

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

  const itemId = item.id;

  // Reset on item change so selection stays cleared until the new item's
  // memberships load; keyed on the id so entry object identity churn from
  // bookmark/unread events doesn't wipe and refetch the same memberships.
  const [lastItemId, setLastItemId] = useState(itemId);
  if (lastItemId !== itemId) {
    setLastItemId(itemId);
    setMemberIds(new Set());
  }

  useEffect(() => {
    if (!itemId) return;
    const controller = new AbortController();
    listEntryTopics(itemId).then((ids) => {
      if (controller.signal.aborted) return;
      setMemberIds(new Set(ids ?? []));
    });
    return () => controller.abort();
  }, [itemId, listEntryTopics, topics]);

  useEffect(() => {
    if (
      !itemId ||
      sortBy !== "relevance" ||
      embeddingStartupState !== "available"
    )
      return;
    let cancelled = false;
    setRelevanceLoading(true);
    void rankTopicsRelatedToItem(itemId).then((results) => {
      if (cancelled) return;
      setSimilarityMap(
        new Map((results ?? []).map((result) => [result.id, result.similarity]))
      );
      setRelevanceLoading(false);
    });
    return () => {
      cancelled = true;
    };
  }, [itemId, sortBy, embeddingStartupState, rankTopicsRelatedToItem]);

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
      const topic = await createTopicWithMember(input, item);
      if (!topic) return false;
      setTopicPinned(topic.id, values.pinned);
      if (currentItemId.current !== item.id) return true;
      setMemberIds((previous) => new Set([...previous, topic.id]));
      return true;
    },
    [createTopicWithMember, item, setTopicPinned, readOnly]
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
          ? await removeTopicItem(topicId, item.id)
          : await addTopicItem(topicId, item);
        if (!topic || currentItemId.current !== item.id) return;
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
    [item, memberIds, addTopicItem, removeTopicItem, readOnly]
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

  if (!hasLoaded) {
    return (
      <Typography className={css.Filter} role="status" variant="body2">
        Loading Topics…
      </Typography>
    );
  }

  return (
    <div className={css.Content}>
      <div className={css.Header}>
        <Typography component="h2" variant="subtitle2" fontWeight={600}>
          Add to Topic
        </Typography>

        <div className={css.HeaderRight}>
          <Select
            aria-label="Sort by"
            value={sortBy}
            size="small"
            variant="standard"
            onChange={handleSortByChange}
          >
            <MenuItem
              value="relevance"
              disabled={embeddingStartupState !== "available"}
            >
              <Typography component="span" variant="body2">
                Relevance
              </Typography>
            </MenuItem>

            <MenuItem value="name">
              <Typography component="span" variant="body2">
                Name
              </Typography>
            </MenuItem>
          </Select>
        </div>
      </div>

      {relevanceLoading && <LinearProgress />}

      {!creating && (
        <div className={css.Filter}>
          <TextField
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
