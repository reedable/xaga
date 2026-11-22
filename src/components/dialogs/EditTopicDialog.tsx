import { useLibraryReadOnly } from "../../hooks/useLibraryReadOnly";
import { useErrorSink } from "../../hooks/useErrorSink";
import {
  Alert,
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
import { useTopicCreationCheck } from "../../hooks/useTopicCreationCheck";
import SimilarTopicsDialog from "./SimilarTopicsDialog";
import {
  TopicFormFields,
  useTopicForm,
  type TopicFormValues,
} from "./TopicFormFields";

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
  ) => Promise<boolean>;
  onOpenTopic: (topic: Topic) => void;
  onDelete: (topic: Topic) => Promise<void>;
};

export default function EditTopicDialog({
  open,
  topic,
  initialTitle,
  onClose,
  onSave,
  onDelete,
  onOpenTopic,
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

  async function saveDraft(draft: TopicFormValues) {
    if (
      await onSave(
        draft.title,
        draft.description,
        draft.threshold,
        draft.pinned
      )
    ) {
      onClose();
    }
  }

  const creation = useTopicCreationCheck(
    open && !topic && !readOnly,
    saveDraft
  );
  const busy = saving || deleting || creation.busy;

  async function handleSubmit(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    if (readOnly || busy || !form.values.title) return;
    if (!topic) {
      creation.submit(form.values);
      return;
    }

    setSaving(true);
    try {
      await saveDraft(form.values);
    } catch (error) {
      pushError(error);
    } finally {
      setSaving(false);
    }
  }

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

  return (
    <>
      <Dialog
        open={open}
        onClose={() => !busy && onClose()}
        fullWidth
        maxWidth="sm"
      >
        <form onSubmit={handleSubmit}>
          <DialogTitle>{topic ? "Edit Topic" : "Add Topic"}</DialogTitle>

          {creation.busy && (
            <p role="status">
              {creation.phase === "checking"
                ? "Checking for similar Topics…"
                : "Saving Topic…"}
            </p>
          )}
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
            {creation.saveError && (
              <Alert severity="error">{creation.saveError}</Alert>
            )}
            <TopicFormFields form={form} busy={busy || readOnly} autoFocus />
          </DialogContent>

          <DialogActions>
            {topic && (
              <Button
                data-testid="delete"
                onClick={handleDelete}
                color="error"
                disabled={busy || readOnly}
                sx={{ mr: "auto" }}
              >
                Delete
              </Button>
            )}
            <Button data-testid="cancel" onClick={onClose} disabled={busy}>
              Cancel
            </Button>
            <Button
              data-testid="save"
              type="submit"
              variant="contained"
              disabled={!form.values.title || busy || readOnly}
            >
              Save
            </Button>
          </DialogActions>
        </form>
      </Dialog>
      {open && creation.warning && (
        <SimilarTopicsDialog
          matches={creation.warning.matches}
          onDismiss={creation.dismiss}
          onConfirm={creation.confirm}
          onOpenTopic={(match) => {
            creation.dismiss();
            onClose();
            onOpenTopic(match);
          }}
        />
      )}
    </>
  );
}
