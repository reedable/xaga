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
  useState,
  type SubmitEvent,
} from "react";
import useAbortController from "../../hooks/useAbortController";
import { useCollections } from "../../hooks/useCollections";
import type { Collection } from "../../types";

type EditCollectionDialogProps = {
  open: boolean;
  collection: Collection | null;
  onClose: () => void;
  onSave: (title: string, pinned: boolean) => Promise<void>;
  onDelete: (collection: Collection) => Promise<void>;
};

export default function EditCollectionDialog({
  open,
  collection,
  onClose,
  onSave,
  onDelete,
}: EditCollectionDialogProps) {
  const [, abortControllerRef] = useAbortController();
  const { pinnedCollectionIds } = useCollections(abortControllerRef);

  const [title, setTitle] = useState("");
  const [pinnedState, setPinnedState] = useState(false);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);

  useEffect(() => {
    if (open) {
      setTitle(collection?.title ?? "");
      setPinnedState(pinnedCollectionIds.includes(collection?.id ?? ""));
    }
  }, [open, collection, pinnedCollectionIds]);

  const handleSubmit = useCallback(
    async (e: SubmitEvent<HTMLFormElement>) => {
      e.preventDefault();
      if (!title.trim()) return;
      setSaving(true);
      try {
        await onSave(title.trim(), pinnedState);
        onClose();
      } finally {
        setSaving(false);
      }
    },
    [onClose, onSave, pinnedState, title]
  );

  const handleDelete = useCallback(async () => {
    if (!collection) return;
    setDeleting(true);
    try {
      await onDelete(collection);
      onClose();
    } finally {
      setDeleting(false);
    }
  }, [collection, onClose, onDelete]);

  const busy = saving || deleting;

  return (
    <Dialog open={open} onClose={onClose} fullWidth maxWidth="sm">
      <form onSubmit={handleSubmit}>
        <DialogTitle>
          {collection ? "Edit Collection" : "New Collection"}
        </DialogTitle>

        <DialogContent
          sx={{
            display: "flex",
            flexDirection: "column",
            gap: 2,
            pt: "16px !important",
          }}
        >
          <TextField
            label="Name"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            required
            autoFocus
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
        </DialogContent>

        <DialogActions>
          {collection && (
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
            {collection ? "Save" : "Create"}
          </Button>
        </DialogActions>
      </form>
    </Dialog>
  );
}
