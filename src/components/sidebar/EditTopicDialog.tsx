import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  TextField,
} from "@mui/material";
import { useState, useEffect, type SubmitEvent } from "react";
import type { Topic } from "../../types";

interface EditTopicDialogProps {
  open: boolean;
  topic: Topic | null;
  onClose: () => void;
  onSave: (title: string, content: string) => Promise<void>;
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
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);

  useEffect(() => {
    if (open) {
      setTitle(topic?.title ?? "");
      setDescription(topic?.content ?? "");
    }
  }, [open, topic]);

  const handleSubmit = async (e: SubmitEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!title.trim()) return;

    setSaving(true);
    try {
      await onSave(title.trim(), description);
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
