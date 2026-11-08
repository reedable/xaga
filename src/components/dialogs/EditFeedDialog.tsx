import { useLibraryReadOnly } from "../../hooks/useLibraryReadOnly";
import { useErrorSink } from "../../hooks/useErrorSink";
import {
  Button,
  Checkbox,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  FormControlLabel,
  TextField,
} from "@mui/material";
import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type SubmitEvent,
} from "react";
import useAbortController from "../../hooks/useAbortController";
import { useFeeds } from "../../hooks/useFeeds";
import type { Feed } from "../../types";

type EditFeedDialogProps = {
  open: boolean;
  feed: Feed | null;
  onClose: () => void;
  onAdd: (url: string, title: string, pinned: boolean) => Promise<void>;
  onSave: (title: string, pinned: boolean) => Promise<void>;
  onDelete: (feed: Feed) => Promise<void>;
};

export default function EditFeedDialog({
  open,
  feed,
  onClose,
  onAdd,
  onSave,
  onDelete,
}: EditFeedDialogProps) {
  const readOnly = useLibraryReadOnly();
  const { pushError } = useErrorSink();
  const [, abortControllerRef] = useAbortController();
  const { pinnedFeedIds } = useFeeds(abortControllerRef);

  const [title, setTitle] = useState("");
  const [url, setUrl] = useState("");
  const [pinnedState, setPinnedState] = useState(false);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);

  const isAdding = feed === null;

  useEffect(() => {
    if (open) {
      setTitle(feed?.title ?? "");
      setUrl(feed?.url ?? "");
      setPinnedState(pinnedFeedIds.includes(feed?.id ?? ""));
    }
  }, [open, feed, pinnedFeedIds]);

  const handleSubmit = useCallback(
    async (e: SubmitEvent<HTMLFormElement>) => {
      e.preventDefault();
      if (readOnly) return;
      setSaving(true);

      try {
        if (isAdding) {
          await onAdd(url.trim(), title.trim(), pinnedState);
        } else {
          if (!title.trim()) return;
          await onSave(title.trim(), pinnedState);
        }
        onClose();
      } catch (error) {
        pushError(error);
      } finally {
        setSaving(false);
      }
    },
    [
      isAdding,
      onAdd,
      onClose,
      onSave,
      pinnedState,
      title,
      url,
      readOnly,
      pushError,
    ]
  );

  const handleDelete = useCallback(async () => {
    if (!feed || readOnly) return;
    setDeleting(true);

    try {
      await onDelete(feed);
      onClose();
    } catch (error) {
      pushError(error);
    } finally {
      setDeleting(false);
    }
  }, [feed, onClose, onDelete, readOnly, pushError]);

  const busy = useMemo(() => saving || deleting, [deleting, saving]);

  const canSubmit = useMemo(
    () => (isAdding ? url.trim().length > 0 : title.trim().length > 0),
    [isAdding, title, url]
  );

  return (
    <Dialog open={open} onClose={onClose} fullWidth maxWidth="sm">
      <form onSubmit={handleSubmit}>
        <DialogTitle>{isAdding ? "Add Feed" : "Edit Feed"}</DialogTitle>

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
          {isAdding ? (
            <>
              <TextField
                disabled={readOnly || saving || deleting}
                label="URL"
                value={url}
                onChange={(e) => setUrl(e.target.value)}
                required
                autoFocus
                fullWidth
                placeholder="https://example.com/feed.xml"
              />

              <TextField
                disabled={readOnly || saving || deleting}
                label="Name (optional)"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                fullWidth
              />
            </>
          ) : (
            <>
              <TextField
                disabled={readOnly || saving || deleting}
                label="Name"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                required
                autoFocus
                fullWidth
              />

              <TextField
                label="URL"
                value={url}
                fullWidth
                disabled
                slotProps={{ input: { readOnly: true } }}
              />
            </>
          )}

          <FormControlLabel
            control={
              <Checkbox
                checked={pinnedState}
                onChange={(e) => setPinnedState(e.target.checked)}
                disabled={busy || readOnly}
              />
            }
            label="Add to Shortcuts"
          />
        </DialogContent>

        <DialogActions>
          {!isAdding && feed && (
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
            disabled={!canSubmit || busy || readOnly}
          >
            {isAdding ? "Add" : "Save"}
          </Button>
        </DialogActions>
      </form>
    </Dialog>
  );
}
