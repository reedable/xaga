import { Add, Remove } from "@mui/icons-material";
import {
  Box,
  Button,
  Checkbox,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  FormControlLabel,
  IconButton,
  TextField,
  Tooltip,
  Typography,
} from "@mui/material";
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type SubmitEvent,
} from "react";
import { TOPIC_THRESHOLD } from "../../consts";
import useAbortController from "../../hooks/useAbortController";
import { useTopics } from "../../hooks/useTopics";
import type { Topic } from "../../types";

type EditTopicDialogProps = {
  open: boolean;
  topic: Topic | null;
  // Prefills the title field when adding a new topic (ignored when editing).
  initialTitle?: string;
  onClose: () => void;
  onSave: (
    title: string,
    content: string,
    threshold: number,
    pinned: boolean
  ) => Promise<void>;
  onDelete: (topic: Topic) => Promise<void>;
};

export default function EditTopicDialog({
  open,
  topic,
  initialTitle,
  onClose,
  onSave,
  onDelete,
}: EditTopicDialogProps) {
  const [, abortControllerRef] = useAbortController();
  const { pinnedTopicIds } = useTopics(abortControllerRef);

  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [thresholdString, setThresholdString] = useState(
    TOPIC_THRESHOLD.toFixed(2)
  );
  const [pinnedState, setPinnedState] = useState(false);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);

  // Initialize the form only on the closed->open transition; re-running on
  // dep changes while open would revert in-progress edits (e.g. during save).
  const wasOpen = useRef(false);

  useEffect(() => {
    if (open && !wasOpen.current) {
      setTitle(topic?.title ?? initialTitle ?? "");
      setDescription(topic?.content ?? "");
      setThresholdString((topic?.threshold ?? TOPIC_THRESHOLD).toFixed(2));
      setPinnedState(pinnedTopicIds.includes(topic?.id ?? ""));
    }
    wasOpen.current = open;
  }, [open, pinnedTopicIds, topic, initialTitle]);

  const parseThreshold = useCallback(() => {
    const thresholdNumber = parseFloat(thresholdString);
    if (isNaN(thresholdNumber)) return TOPIC_THRESHOLD;
    return Math.max(-1, Math.min(1, thresholdNumber));
  }, [thresholdString]);

  const handleDecrement = useCallback(() => {
    const next = Math.max(-1, parseFloat((parseThreshold() - 0.01).toFixed(2)));
    setThresholdString(next.toFixed(2));
  }, [parseThreshold]);

  const handleIncrement = useCallback(() => {
    const next = Math.min(1, parseFloat((parseThreshold() + 0.01).toFixed(2)));
    setThresholdString(next.toFixed(2));
  }, [parseThreshold]);

  const handleThresholdBlur = useCallback(() => {
    setThresholdString(parseThreshold().toFixed(2));
  }, [parseThreshold]);

  const handleSubmit = useCallback(
    async (e: SubmitEvent<HTMLFormElement>) => {
      e.preventDefault();
      if (!title.trim()) return;

      setSaving(true);
      try {
        await onSave(title.trim(), description, parseThreshold(), pinnedState);
        onClose();
      } finally {
        setSaving(false);
      }
    },
    [description, onClose, onSave, parseThreshold, pinnedState, title]
  );

  const handleDelete = useCallback(async () => {
    if (!topic) return;
    setDeleting(true);

    try {
      await onDelete(topic);
      onClose();
    } finally {
      setDeleting(false);
    }
  }, [onClose, onDelete, topic]);

  const busy = saving || deleting;

  return (
    <Dialog open={open} onClose={onClose} fullWidth maxWidth="sm">
      <form onSubmit={handleSubmit}>
        <DialogTitle>{topic ? "Edit Topic" : "Add Topic"}</DialogTitle>

        <DialogContent
          sx={{
            display: "flex",
            flexDirection: "column",
            gap: 2,
            pt: "16px !important",
          }}
        >
          <TextField
            label="Title"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            required
            autoFocus
            fullWidth
          />

          <TextField
            label="Description"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            multiline
            minRows={4}
            fullWidth
          />

          <FormControlLabel
            control={
              <Checkbox
                checked={pinnedState}
                onChange={(e) => setPinnedState(e.target.checked)}
                disabled={busy}
              />
            }
            label="Add to Shortcuts"
          />

          <Box>
            <Typography
              variant="caption"
              color="text.secondary"
              display="block"
              gutterBottom
            >
              Similarity threshold
            </Typography>

            <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
              <Tooltip title="Decrease threshold">
                <IconButton
                  size="small"
                  onClick={handleDecrement}
                  disabled={busy}
                  aria-label="Decrease threshold"
                >
                  <Remove fontSize="small" />
                </IconButton>
              </Tooltip>

              <TextField
                value={thresholdString}
                onChange={(e) => setThresholdString(e.target.value)}
                onBlur={handleThresholdBlur}
                size="small"
                sx={{ width: 80 }}
                slotProps={{ htmlInput: { style: { textAlign: "center" } } }}
              />

              <Tooltip title="Increase threshold">
                <IconButton
                  size="small"
                  onClick={handleIncrement}
                  disabled={busy}
                  aria-label="Increase threshold"
                >
                  <Add fontSize="small" />
                </IconButton>
              </Tooltip>
            </Box>
          </Box>
        </DialogContent>

        <DialogActions>
          {topic && (
            <Button
              onClick={handleDelete}
              color="error"
              disabled={busy}
              sx={{ mr: "auto" }}
            >
              Delete
            </Button>
          )}
          <Button onClick={onClose} disabled={busy}>
            Cancel
          </Button>
          <Button
            type="submit"
            variant="contained"
            disabled={!title.trim() || busy}
          >
            Save
          </Button>
        </DialogActions>
      </form>
    </Dialog>
  );
}
