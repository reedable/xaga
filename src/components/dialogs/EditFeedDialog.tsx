import { useLibraryReadOnly } from "../../hooks/useLibraryReadOnly";
import { useErrorSink } from "../../hooks/useErrorSink";
import {
  Alert,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  TextField,
} from "@mui/material";
import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  useRef,
  type SubmitEvent,
} from "react";
import ShortcutDestination, {
  shortcutSaveError,
  useShortcutSelection,
} from "./ShortcutDestination";
import { errorMessage } from "../../platform/errors";
import type { Feed } from "../../types";

type EditFeedDialogProps = {
  open: boolean;
  feed: Feed | null;
  onClose: () => void;
  onAdd: (url: string, savedId?: string) => Promise<Feed>;
  onRename: (id: string, title: string) => Promise<Feed>;
  onSaved: (feed: Feed, added: boolean) => Promise<void>;
  onDelete: (feed: Feed) => Promise<void>;
};

export default function EditFeedDialog({
  open,
  feed,
  onClose,
  onAdd,
  onRename,
  onSaved,
  onDelete,
}: EditFeedDialogProps) {
  const readOnly = useLibraryReadOnly();
  const { pushError } = useErrorSink();
  const shortcut = useShortcutSelection(
    feed ? { type: "feed", id: feed.id } : undefined
  );
  const resetShortcut = shortcut.reset;
  const inFlight = useRef(false);
  const saved = useRef<Feed | null>(null);
  const [saveError, setSaveError] = useState<string>();
  const [refreshId, setRefreshId] = useState<string>();
  const [entitySaved, setEntitySaved] = useState(false);
  const [feedAdded, setFeedAdded] = useState(false);

  const [title, setTitle] = useState("");
  const [url, setUrl] = useState("");
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);

  const isAdding = feed === null;

  const wasOpen = useRef(false);
  useEffect(() => {
    if (open && !wasOpen.current) {
      saved.current = null;
      setRefreshId(undefined);
      setSaveError(undefined);
      setEntitySaved(false);
      setFeedAdded(false);
      resetShortcut();
      setTitle(feed?.title ?? "");
      setUrl(feed?.url ?? "");
    }
    wasOpen.current = open;
  }, [open, feed, resetShortcut]);

  async function handleSubmit(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    if (readOnly || inFlight.current || deleting) return;
    if (isAdding ? !url.trim() : !title.trim()) return;
    inFlight.current = true;
    setSaving(true);
    setSaveError(undefined);

    try {
      let result = saved.current ?? feed;
      if (!result) {
        try {
          result = await onAdd(url.trim(), refreshId);
          saved.current = result;
          setFeedAdded(true);
          setRefreshId(undefined);
        } catch (error) {
          if (
            typeof error === "object" &&
            error &&
            "savedFeedId" in error &&
            typeof error.savedFeedId === "string"
          ) {
            setRefreshId(error.savedFeedId);
            throw {
              code: "feed:saved-refresh",
              message:
                "The Feed was saved, but its articles couldn't be refreshed. Retry to refresh it.",
              detail: "",
            };
          }
          throw error;
        }
      }

      if (!entitySaved && title.trim() && title.trim() !== result.title) {
        try {
          result = await onRename(result.id, title.trim());
          saved.current = result;
        } catch (cause) {
          if (isAdding)
            throw {
              code: "app:error",
              message:
                "The Feed was saved, but its name couldn't be saved. Retry to save the name.",
              detail: "",
            };
          throw cause;
        }
      }
      setEntitySaved(true);
      try {
        await shortcut.save({ type: "feed", id: result.id });
      } catch (cause) {
        throw shortcutSaveError("Feed", cause);
      }
      await onSaved(result, isAdding);
      onClose();
    } catch (error) {
      setSaveError(errorMessage(error));
      pushError(error);
    } finally {
      inFlight.current = false;
      setSaving(false);
    }
  }

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
    <Dialog
      open={open}
      onClose={() => {
        if (!inFlight.current && !deleting) onClose();
      }}
      fullWidth
      maxWidth="sm"
    >
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
                data-testid="feed-url"
                disabled={
                  busy || readOnly || refreshId !== undefined || feedAdded
                }
                label="URL"
                value={url}
                onChange={(e) => setUrl(e.target.value)}
                required
                autoFocus
                fullWidth
                placeholder="https://example.com/feed.xml"
              />

              <TextField
                data-testid="collection-name"
                disabled={busy || readOnly || entitySaved}
                label="Name (optional)"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                fullWidth
              />
            </>
          ) : (
            <>
              <TextField
                disabled={busy || readOnly || entitySaved}
                label="Name"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                required
                autoFocus
                fullWidth
              />

              <TextField
                data-testid="feed-url"
                label="URL"
                value={url}
                fullWidth
                disabled
                slotProps={{ input: { readOnly: true } }}
              />
            </>
          )}

          {saveError && <Alert severity="error">{saveError}</Alert>}
          <ShortcutDestination
            value={shortcut.value}
            onChange={shortcut.change}
            disabled={busy || readOnly}
          />
        </DialogContent>

        <DialogActions>
          {!isAdding && feed && (
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
            data-testid={feed ? "save" : "add"}
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
