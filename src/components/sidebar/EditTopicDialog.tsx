import { Add, Remove } from "@mui/icons-material";
import {
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  IconButton,
  TextField,
  Typography,
} from "@mui/material";
import { useState, useEffect, type SubmitEvent } from "react";
import { TOPIC_THRESHOLD } from "../../consts";
import type { Topic } from "../../types";

interface EditTopicDialogProps {
  open: boolean;
  topic: Topic | null;
  onClose: () => void;
  onSave: (title: string, content: string, threshold: number) => Promise<void>;
  onDelete: (topic: Topic) => Promise<void>;
}

export default function EditTopicDialog({
  open,
  topic,
  onClose,
  onSave,
  onDelete,
}: EditTopicDialogProps) {
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [thresholdText, setThresholdText] = useState(TOPIC_THRESHOLD.toFixed(2));
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);

  useEffect(() => {
    if (open) {
      setTitle(topic?.title ?? "");
      setDescription(topic?.content ?? "");
      setThresholdText((topic?.threshold ?? TOPIC_THRESHOLD).toFixed(2));
    }
  }, [open, topic]);

  const parseThreshold = () => {
    const v = parseFloat(thresholdText);
    if (isNaN(v)) return TOPIC_THRESHOLD;
    return Math.max(-1, Math.min(1, v));
  };

  const handleDecrement = () => {
    const next = Math.max(-1, parseFloat((parseThreshold() - 0.01).toFixed(2)));
    setThresholdText(next.toFixed(2));
  };

  const handleIncrement = () => {
    const next = Math.min(1, parseFloat((parseThreshold() + 0.01).toFixed(2)));
    setThresholdText(next.toFixed(2));
  };

  const handleThresholdBlur = () => {
    setThresholdText(parseThreshold().toFixed(2));
  };

  const handleSubmit = async (e: SubmitEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!title.trim()) return;

    setSaving(true);
    try {
      await onSave(title.trim(), description, parseThreshold());
      onClose();
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async () => {
    if (!topic) return;
    setDeleting(true);
    try {
      await onDelete(topic);
      onClose();
    } finally {
      setDeleting(false);
    }
  };

  const busy = saving || deleting;

  return (
    <Dialog open={open} onClose={onClose} fullWidth maxWidth="sm">
      <form onSubmit={handleSubmit}>
        <DialogTitle>{topic ? "Edit Topic" : "Add Topic"}</DialogTitle>

        <DialogContent sx={{ display: "flex", flexDirection: "column", gap: 2, pt: "16px !important" }}>
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
          <Box>
            <Typography variant="caption" color="text.secondary" display="block" gutterBottom>
              Similarity threshold
            </Typography>
            <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
              <IconButton size="small" onClick={handleDecrement} disabled={busy}>
                <Remove fontSize="small" />
              </IconButton>
              <TextField
                value={thresholdText}
                onChange={(e) => setThresholdText(e.target.value)}
                onBlur={handleThresholdBlur}
                size="small"
                sx={{ width: 80 }}
                slotProps={{ htmlInput: { style: { textAlign: "center" } } }}
              />
              <IconButton size="small" onClick={handleIncrement} disabled={busy}>
                <Add fontSize="small" />
              </IconButton>
            </Box>
          </Box>
        </DialogContent>

        <DialogActions>
          {topic && (
            <Button onClick={handleDelete} color="error" disabled={busy} sx={{ mr: "auto" }}>
              Delete
            </Button>
          )}
          <Button onClick={onClose} disabled={busy}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={!title.trim() || busy}>
            Save
          </Button>
        </DialogActions>
      </form>
    </Dialog>
  );
}
