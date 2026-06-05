import { Close, CreateNewFolder } from "@mui/icons-material";
import {
  Badge,
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
} from "@mui/material";
import { motion } from "framer-motion";
import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useMemo,
  useState,
} from "react";
import useAbortController from "../../hooks/useAbortController";
import { useEvents } from "../../hooks/useEvents";
import { useTopics } from "../../hooks/useTopics";
import type { Annotation, Entry } from "../../types";
import css from "./TopicMenuButton.module.css";

const MotionListItem = motion(ListItem);

interface TopicMenuButtonProps {
  item: Entry | Annotation | null;
  disabled?: boolean;
}

export interface TopicMenuButtonHandle {
  saveToTopics: (item: Entry | Annotation) => Promise<void>;
}

export const TopicMenuButton = forwardRef<
  TopicMenuButtonHandle,
  TopicMenuButtonProps
>(function TopicMenuButton({ item, disabled }, ref) {
  const [, abortControllerRef] = useAbortController();
  const { onAnnotationRemoved } = useEvents();
  const {
    topics,
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
  const [sortBy, setSortBy] = useState<"name" | "relevance">("relevance");
  const [similarityMap, setSimilarityMap] = useState<Map<string, number>>(
    new Map()
  );
  const [relevanceLoading, setRelevanceLoading] = useState(false);

  useEffect(() => {
    if (!item) return;
    listEntryTopics(item.id).then((ids) => {
      setMemberIds(new Set(ids ?? []));
    });
  }, [item, listEntryTopics]);

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
      async saveToTopics(item: Entry | Annotation) {
        await Promise.all(
          [...selectedTopicIds].map((topicId) => addTopicItem(topicId, item))
        );
        setSelectedTopicIds(new Set());
      },
    }),
    [selectedTopicIds, addTopicItem]
  );

  const handleOpen = useCallback(async () => {
    setOpen(true);
    if (item) {
      const ids = await listEntryTopics(item.id);
      setMemberIds(new Set(ids ?? []));
      if (sortBy === "relevance") {
        setRelevanceLoading(true);
        const results = await rankTopicsRelatedToItem(item.id);
        setSimilarityMap(
          new Map((results ?? []).map((r) => [r.id, r.similarity]))
        );
        setRelevanceLoading(false);
      }
    }
  }, [item, listEntryTopics, sortBy, rankTopicsRelatedToItem]);

  const handleClose = useCallback(() => {
    setOpen(false);
    setFilterText("");
  }, []);

  const handleSortByChange = useCallback(
    async (event: SelectChangeEvent) => {
      const value = event.target.value as "name" | "relevance";
      setSortBy(value);
      if (value === "relevance" && item) {
        setRelevanceLoading(true);
        const results = await rankTopicsRelatedToItem(item.id);
        setSimilarityMap(
          new Map((results ?? []).map((r) => [r.id, r.similarity]))
        );
        setRelevanceLoading(false);
      }
    },
    [item, rankTopicsRelatedToItem]
  );

  const handleToggle = useCallback(
    async (topicId: string) => {
      if (!item) {
        setSelectedTopicIds((prev) => {
          const next = new Set(prev);
          if (next.has(topicId)) next.delete(topicId);
          else next.add(topicId);
          return next;
        });
        return;
      }
      if (memberIds.has(topicId)) {
        await removeTopicItem(topicId, item.id);
        setMemberIds((prev) => {
          const next = new Set(prev);
          next.delete(topicId);
          return next;
        });
      } else {
        await addTopicItem(topicId, item);
        setMemberIds((prev) => new Set([...prev, topicId]));
      }
    },
    [item, memberIds, addTopicItem, removeTopicItem]
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
        return (similarityMap.get(b.id) ?? 0) - (similarityMap.get(a.id) ?? 0);
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

  return (
    <>
      <Tooltip title="Add to Topic">
        <IconButton
          size="small"
          onClick={handleOpen}
          disabled={topics.length === 0 || disabled}
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

      <Drawer anchor="right" open={open} onClose={handleClose}>
        <div className={css.DrawerContent}>
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
                <MenuItem value="relevance" disabled={!item}>
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

              <Tooltip title="Close">
                <IconButton
                  size="small"
                  onClick={handleClose}
                  aria-label="Close"
                >
                  <Close fontSize="small" />
                </IconButton>
              </Tooltip>
            </div>
          </div>

          {relevanceLoading && <LinearProgress />}

          <div className={css.DrawerFilter}>
            <TextField
              placeholder="Filter topics…"
              size="small"
              fullWidth
              value={filterText}
              onChange={(e) => setFilterText(e.target.value)}
              autoFocus
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
          <List className={css.DrawerList} disablePadding>
            {filteredTopics.map((t) => (
              <MotionListItem
                key={t.id}
                layout
                transition={{ duration: 0.3, ease: "easeOut" }}
                disablePadding
              >
                <ListItemButton onClick={() => handleToggle(t.id)} dense>
                  <Checkbox
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
          </List>
        </div>
      </Drawer>
    </>
  );
});
