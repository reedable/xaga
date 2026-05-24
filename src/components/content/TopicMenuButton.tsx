import { Close, CreateNewFolder } from "@mui/icons-material";
import {
  Badge,
  Checkbox,
  Divider,
  Drawer,
  IconButton,
  InputAdornment,
  LinearProgress,
  List,
  ListItem,
  ListItemButton,
  ListItemText,
  TextField,
  ToggleButton,
  ToggleButtonGroup,
  Tooltip,
  Typography,
} from "@mui/material";
import { motion } from "framer-motion";
import { forwardRef, useCallback, useEffect, useImperativeHandle, useMemo, useState } from "react";
import useAbortController from "../../hooks/useAbortController";
import { useEvents } from "../../hooks/useEvents";
import { useTopics } from "../../hooks/useTopics";
import type { Annotation, Entry } from "../../types";
import css from "./TopicMenuButton.module.css";

const MotionListItem = motion(ListItem);

interface TopicMenuButtonProps {
  item: Entry | Annotation | null;
}

export interface TopicMenuButtonHandle {
  saveToTopics: (item: Entry | Annotation) => Promise<void>;
}

export const TopicMenuButton = forwardRef<TopicMenuButtonHandle, TopicMenuButtonProps>(
  function TopicMenuButton({ item }, ref) {
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
    const [selectedTopicIds, setSelectedTopicIds] = useState<Set<string>>(new Set());
    const [filterText, setFilterText] = useState("");
    const [sortBy, setSortBy] = useState<"name" | "relevance">("name");
    const [similarityMap, setSimilarityMap] = useState<Map<string, number>>(new Map());
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

    useImperativeHandle(ref, () => ({
      async saveToTopics(item: Entry | Annotation) {
        await Promise.all(
          [...selectedTopicIds].map((topicId) => addTopicItem(topicId, item))
        );
        setSelectedTopicIds(new Set());
      },
    }), [selectedTopicIds, addTopicItem]);

    const handleOpen = useCallback(async () => {
      setOpen(true);
      if (item) {
        const ids = await listEntryTopics(item.id);
        setMemberIds(new Set(ids ?? []));
      }
    }, [item, listEntryTopics]);

    const handleClose = useCallback(() => {
      setOpen(false);
      setFilterText("");
    }, []);

    const handleSortByChange = useCallback(
      async (_: React.MouseEvent, value: "name" | "relevance" | null) => {
        if (!value) return;
        setSortBy(value);
        if (value === "relevance" && item) {
          setRelevanceLoading(true);
          const results = await rankTopicsRelatedToItem(item.id);
          setSimilarityMap(new Map((results ?? []).map((r) => [r.id, r.similarity])));
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
    }, [sortedTopics, filterText, item, memberIds, selectedTopicIds, sortBy, similarityMap]);

    return (
      <>
        <Tooltip title="Add to Topic">
          <span>
            <IconButton
              size="small"
              onClick={handleOpen}
              disabled={topics.length === 0}
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
          </span>
        </Tooltip>

        <Drawer anchor="right" open={open} onClose={handleClose}>
          <div className={css.DrawerContent}>
            <div className={css.DrawerHeader}>
              <Typography variant="subtitle1" fontWeight={600}>
                Add to Topic
              </Typography>
              <div className={css.DrawerHeaderRight}>
                <ToggleButtonGroup
                  value={sortBy}
                  exclusive
                  size="small"
                  onChange={handleSortByChange}
                >
                  <ToggleButton value="name">Name</ToggleButton>
                  <ToggleButton value="relevance" disabled={!item}>Relevance</ToggleButton>
                </ToggleButtonGroup>
                <IconButton size="small" onClick={handleClose}>
                  <Close fontSize="small" />
                </IconButton>
              </div>
            </div>
            <Divider />
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
                slotProps={{
                  htmlInput: { spellCheck: false },
                  input: {
                    endAdornment: filterText && (
                      <InputAdornment position="end">
                        <IconButton size="small" edge="end" onClick={() => setFilterText("")}>
                          <Close fontSize="small" />
                        </IconButton>
                      </InputAdornment>
                    ),
                  },
                }}
              />
            </div>
            <List className={css.DrawerList} disablePadding>
              {filteredTopics.map((t) => (
                <MotionListItem key={t.id} layout transition={{ duration: 0.3, ease: "easeOut" }} disablePadding>
                  <ListItemButton onClick={() => handleToggle(t.id)} dense>
                    <Checkbox
                      checked={item ? memberIds.has(t.id) : selectedTopicIds.has(t.id)}
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
  }
);
