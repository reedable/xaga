import { Close, CreateNewFolder } from "@mui/icons-material";
import {
  Badge,
  Box,
  Button,
  Checkbox,
  Drawer,
  IconButton,
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
  Tooltip,
  Typography,
  useMediaQuery,
} from "@mui/material";
import { motion } from "framer-motion";
import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef,
  useState,
} from "react";
import useAbortController from "../../hooks/useAbortController";
import { useEvents } from "../../hooks/useEvents";
import { useLibraryReadOnly } from "../../hooks/useLibraryReadOnly";
import { useTopics } from "../../hooks/useTopics";
import { useEmbeddingStartupState } from "../../hooks/useEmbeddingStartupState";
import type { Annotation, Entry } from "../../types";
import { TopicFormFields, useTopicForm } from "../dialogs/TopicFormFields";
import css from "./TopicMenuButton.module.css";

const MotionListItem = motion(ListItem);

interface TopicMenuButtonProps {
  item: Entry | Annotation | null;
  disabled?: boolean;
  embedded?: boolean;
}

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

export interface TopicMenuButtonHandle {
  addToTopics: (item: Entry | Annotation) => Promise<void>;
}

const TopicPicker = forwardRef<TopicMenuButtonHandle, TopicMenuButtonProps>(
  function TopicPicker({ item, disabled, embedded = false }, ref) {
    const [, abortControllerRef] = useAbortController();
    const readOnly = useLibraryReadOnly();
    const animate = useMediaQuery("(prefers-reduced-motion: no-preference)");
    const mutationPending = useRef(false);
    const [mutating, setMutating] = useState(false);
    const currentItemId = useRef(item?.id);
    useEffect(() => {
      currentItemId.current = item?.id;
    }, [item?.id]);
    const embeddingStartupState = useEmbeddingStartupState();
    const { onAnnotationRemoved } = useEvents();
    const {
      topics,
      createTopic,
      createTopicWithMember,
      setTopicPinned,
      listEntryTopics,
      addTopicItem,
      removeTopicItem,
      rankTopicsRelatedToItem,
    } = useTopics(abortControllerRef);

    const [open, setOpen] = useState(false);
    const [memberIds, setMemberIds] = useState<Set<string>>(new Set());
    const [selectedTopicIds, setSelectedTopicIds] = useState<Set<string>>(
      new Set()
    );
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

    const itemId = item?.id;

    // Reset on item change so the badge stays cleared until the new item's
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
      if (!item) return;
      return onAnnotationRemoved((id) => {
        if (id === item.id) {
          setMemberIds(new Set());
          setOpen(false);
          setFilterText("");
        }
      });
    }, [item, onAnnotationRemoved]);

    useImperativeHandle(
      ref,
      () => ({
        async addToTopics(item: Entry | Annotation) {
          if (readOnly) return;
          const results = await Promise.all(
            [...selectedTopicIds].map(async (topicId) => ({
              topicId,
              topic: await addTopicItem(topicId, item),
            }))
          );
          const addedIds = new Set(
            results.filter(({ topic }) => topic).map(({ topicId }) => topicId)
          );
          setSelectedTopicIds((previous) => {
            const next = new Set(previous);
            for (const topicId of addedIds) next.delete(topicId);
            return next;
          });
        },
      }),
      [selectedTopicIds, addTopicItem, readOnly]
    );

    const handleOpen = useCallback(async () => {
      setOpen(true);
      if (item) {
        const ids = await listEntryTopics(item.id);
        if (currentItemId.current !== item.id) return;
        setMemberIds(new Set(ids ?? []));
        if (sortBy === "relevance" && embeddingStartupState === "available") {
          setRelevanceLoading(true);
          const results = await rankTopicsRelatedToItem(item.id);
          if (currentItemId.current !== item.id) return;
          setSimilarityMap(
            new Map((results ?? []).map((r) => [r.id, r.similarity]))
          );
          setRelevanceLoading(false);
        }
      }
    }, [
      item,
      listEntryTopics,
      sortBy,
      rankTopicsRelatedToItem,
      embeddingStartupState,
    ]);

    useEffect(() => {
      if (
        !embedded ||
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
          new Map(
            (results ?? []).map((result) => [result.id, result.similarity])
          )
        );
        setRelevanceLoading(false);
      });
      return () => {
        cancelled = true;
      };
    }, [
      embedded,
      itemId,
      sortBy,
      embeddingStartupState,
      rankTopicsRelatedToItem,
    ]);

    const handleClose = useCallback(() => {
      setOpen(false);
      setFilterText("");
      setCreating(false);
    }, []);

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
        const topic = item
          ? await createTopicWithMember(input, item)
          : await createTopic(input);
        if (!topic) return false;
        setTopicPinned(topic.id, values.pinned);
        if (currentItemId.current !== item?.id) return true;
        if (item) setMemberIds((previous) => new Set([...previous, topic.id]));
        else
          setSelectedTopicIds((previous) => new Set([...previous, topic.id]));
        return true;
      },
      [createTopic, createTopicWithMember, item, setTopicPinned, readOnly]
    );

    const handleSortByChange = useCallback(
      async (event: SelectChangeEvent) => {
        const value = event.target.value as "name" | "relevance";
        setSortBy(value);
        if (value === "relevance" && item && !embedded) {
          setRelevanceLoading(true);
          const results = await rankTopicsRelatedToItem(item.id);
          if (currentItemId.current !== item.id) return;
          setSimilarityMap(
            new Map((results ?? []).map((r) => [r.id, r.similarity]))
          );
          setRelevanceLoading(false);
        }
      },
      [item, rankTopicsRelatedToItem, embedded]
    );

    const handleToggle = useCallback(
      async (topicId: string) => {
        if (readOnly) return;
        if (!item) {
          setSelectedTopicIds((prev) => {
            const next = new Set(prev);
            if (next.has(topicId)) next.delete(topicId);
            else next.add(topicId);
            return next;
          });
          return;
        }
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
      const checkedIds = item ? memberIds : selectedTopicIds;
      const base = filterText
        ? sortedTopics.filter((t) =>
            t.title.toLowerCase().includes(filterText.toLowerCase())
          )
        : sortedTopics;
      return [...base].sort((a, b) => {
        const aChecked = checkedIds.has(a.id);
        const bChecked = checkedIds.has(b.id);
        if (aChecked !== bChecked) return aChecked ? -1 : 1;
        if (sortBy === "relevance") {
          return (
            (similarityMap.get(b.id) ?? 0) - (similarityMap.get(a.id) ?? 0)
          );
        }
        return 0;
      });
    }, [
      sortedTopics,
      filterText,
      item,
      memberIds,
      selectedTopicIds,
      sortBy,
      similarityMap,
    ]);

    const content = (
      <div className={embedded ? css.EmbeddedContent : css.DrawerContent}>
        <div className={css.DrawerHeader}>
          <Typography component="h2" variant="subtitle2" fontWeight={600}>
            Add to Topic
          </Typography>

          <div className={css.DrawerHeaderRight}>
            <Select
              aria-label="Sort by"
              value={sortBy}
              size="small"
              variant="standard"
              onChange={handleSortByChange}
            >
              <MenuItem
                value="relevance"
                disabled={!item || embeddingStartupState !== "available"}
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

            {!embedded && (
              <Tooltip title="Close">
                <IconButton
                  size="small"
                  onClick={handleClose}
                  aria-label="Close"
                >
                  <Close fontSize="small" />
                </IconButton>
              </Tooltip>
            )}
          </div>
        </div>

        {relevanceLoading && <LinearProgress />}

        {!creating && (
          <div className={css.DrawerFilter}>
            <TextField
              placeholder="Filter topics…"
              size="small"
              fullWidth
              value={filterText}
              inputRef={filterInputRef}
              onChange={(e) => setFilterText(e.target.value)}
              autoFocus={!embedded}
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
          <List className={css.DrawerList} disablePadding>
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
                  aria-checked={
                    item ? memberIds.has(t.id) : selectedTopicIds.has(t.id)
                  }
                >
                  <Checkbox
                    tabIndex={-1}
                    disabled={readOnly || mutating}
                    slotProps={{ input: { "aria-hidden": true } }}
                    checked={
                      item ? memberIds.has(t.id) : selectedTopicIds.has(t.id)
                    }
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

    if (embedded) return content;

    return (
      <>
        <Tooltip title="Add to Topic">
          <IconButton
            size="small"
            onClick={handleOpen}
            disabled={disabled}
            aria-label="Add to Topic"
          >
            <Badge
              badgeContent={item ? memberIds.size : selectedTopicIds.size}
              color="primary"
              overlap="circular"
              sx={{
                "& .MuiBadge-badge": {
                  fontSize: "0.6rem",
                  height: 13,
                  minWidth: 13,
                  padding: "0 2px",
                },
              }}
            >
              <CreateNewFolder fontSize="small" />
            </Badge>
          </IconButton>
        </Tooltip>

        <Drawer
          anchor="right"
          open={open}
          onClose={handleClose}
          transitionDuration={animate ? undefined : 0}
        >
          {content}
        </Drawer>
      </>
    );
  }
);

export const TopicMenuButton = forwardRef<
  TopicMenuButtonHandle,
  Omit<TopicMenuButtonProps, "embedded">
>(function TopicMenuButton(props, ref) {
  return <TopicPicker {...props} ref={ref} />;
});

export function ArticleTopicMembership({ article }: { article: Entry }) {
  return <TopicPicker item={article} embedded />;
}
