import { useLibraryReadOnly } from "../../hooks/useLibraryReadOnly";
import { useErrorSink } from "../../hooks/useErrorSink";
import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
} from "@mui/material";
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type SubmitEvent,
} from "react";
import useAbortController from "../../hooks/useAbortController";
import { useTopics } from "../../hooks/useTopics";
import type { Topic } from "../../types";
import { TopicFormFields, useTopicForm } from "./TopicFormFields";

type EditTopicDialogProps = {
  open: boolean;
  topic: Topic | null;
  // Prefills the title field when adding a new topic (ignored when editing).
  initialTitle?: string;
  onClose: () => void;
  onSave: (
    title: string,
    description: string,
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
  const readOnly = useLibraryReadOnly();
  const { pushError } = useErrorSink();
  const [, abortControllerRef] = useAbortController();
  const { pinnedTopicIds } = useTopics(abortControllerRef);

  const form = useTopicForm();
  const resetForm = form.reset;
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);

  // Initialize the form only on the closed->open transition; re-running on
  // dep changes while open would revert in-progress edits (e.g. during save).
  const wasOpen = useRef(false);

  useEffect(() => {
    if (open && !wasOpen.current) {
      resetForm({
        title: topic?.title ?? initialTitle,
        description: topic?.description,
        threshold: topic?.threshold,
        pinned: pinnedTopicIds.includes(topic?.id ?? ""),
      });
    }
    wasOpen.current = open;
  }, [open, pinnedTopicIds, topic, initialTitle, resetForm]);

  const handleSubmit = useCallback(
    async (e: SubmitEvent<HTMLFormElement>) => {
      e.preventDefault();
      if (readOnly) return;
      if (!form.values.title) return;

      setSaving(true);
      try {
        await onSave(
          form.values.title,
          form.values.description,
          form.values.threshold,
          form.values.pinned
        );
        onClose();
      } catch (error) {
        pushError(error);
      } finally {
        setSaving(false);
      }
    },
    [form.values, onClose, onSave, readOnly, pushError]
  );

  const handleDelete = useCallback(async () => {
    if (!topic || readOnly) return;
    setDeleting(true);

    try {
      await onDelete(topic);
      onClose();
    } catch (error) {
      pushError(error);
    } finally {
      setDeleting(false);
    }
  }, [onClose, onDelete, topic, readOnly, pushError]);

  const busy = saving || deleting;

  return (
    <Dialog open={open} onClose={onClose} fullWidth maxWidth="sm">
      <form onSubmit={handleSubmit}>
        <DialogTitle>{topic ? "Edit Topic" : "Add Topic"}</DialogTitle>

        {readOnly && (
          <p role="status">
            This library is read-only. Changes are unavailable.
          </p>
        )}
        <DialogContent
          sx={{
            display: "flex",
            flexDirection: "column",
            gap: 2,
            pt: "1rem !important",
          }}
        >
          <TopicFormFields form={form} busy={busy || readOnly} autoFocus />
        </DialogContent>

        <DialogActions>
          {topic && (
            <Button
              onClick={handleDelete}
              color="error"
              disabled={busy || readOnly}
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
            disabled={!form.values.title || busy || readOnly}
          >
            Save
          </Button>
        </DialogActions>
      </form>
    </Dialog>
  );
}
