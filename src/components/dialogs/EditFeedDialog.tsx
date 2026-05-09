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
      setSaving(true);

      try {
        if (isAdding) {
          await onAdd(url.trim(), title.trim(), pinnedState);
        } else {
          if (!title.trim()) return;
          await onSave(title.trim(), pinnedState);
        }
        onClose();
      } finally {
        setSaving(false);
      }
    },
    [isAdding, onAdd, onClose, onSave, pinnedState, title, url]
  );

  const handleDelete = useCallback(async () => {
    if (!feed) return;
    setDeleting(true);

    try {
      await onDelete(feed);
      onClose();
    } finally {
      setDeleting(false);
    }
  }, [feed, onClose, onDelete]);

  const busy = useMemo(() => saving || deleting, [deleting, saving]);

  const canSubmit = useMemo(
    () => (isAdding ? url.trim().length > 0 : title.trim().length > 0),
    [isAdding, title, url]
  );

  return (
    <Dialog open={open} onClose={onClose} fullWidth maxWidth="sm">
      <form onSubmit={handleSubmit}>
        <DialogTitle>{isAdding ? "Add Feed" : "Edit Feed"}</DialogTitle>

        <DialogContent
          sx={{
            display: "flex",
            flexDirection: "column",
            gap: 2,
            pt: "16px !important",
          }}
        >
          {isAdding ? (
            <>
              <TextField
                label="URL"
                value={url}
                onChange={(e) => setUrl(e.target.value)}
                required
                autoFocus
                fullWidth
                placeholder="https://example.com/feed.xml"
              />

              <TextField
                label="Name (optional)"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                fullWidth
              />
            </>
          ) : (
            <>
              <TextField
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
                disabled={busy}
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
            disabled={!canSubmit || busy}
          >
            {isAdding ? "Add" : "Save"}
          </Button>
        </DialogActions>
      </form>
    </Dialog>
  );
}
