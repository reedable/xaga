import {
  Alert,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  TextField,
} from "@mui/material";
import { useEffect, useRef, useState, type SubmitEvent } from "react";
import ShortcutDestination, {
  useShortcutSelection,
} from "./ShortcutDestination";
import { useLibraryReadOnly } from "../../hooks/useLibraryReadOnly";
import { useErrorSink } from "../../hooks/useErrorSink";
import { errorMessage } from "../../platform/errors";
import type { Site } from "../../types";

export default function EditSiteDialog({
  site,
  onClose,
}: {
  site: Site | null;
  onClose: () => void;
}) {
  const readOnly = useLibraryReadOnly();
  const { pushError } = useErrorSink();
  const hostname = site?.id;
  const target =
    hostname === undefined ? undefined : ({ type: "site", hostname } as const);
  const shortcut = useShortcutSelection(target);
  const resetShortcut = shortcut.reset;
  const [saveError, setSaveError] = useState<string>();
  const [saving, setSaving] = useState(false);
  const inFlight = useRef(false);
  useEffect(() => {
    resetShortcut();
    setSaveError(undefined);
  }, [hostname, resetShortcut]);
  const submit = async (event: SubmitEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!target || readOnly || inFlight.current) return;
    inFlight.current = true;
    setSaving(true);
    setSaveError(undefined);
    try {
      await shortcut.save(target);
      onClose();
    } catch (error) {
      setSaveError(errorMessage(error));
      pushError(error);
    } finally {
      inFlight.current = false;
      setSaving(false);
    }
  };
  return (
    <Dialog
      open={site !== null}
      onClose={() => {
        if (!saving) onClose();
      }}
      fullWidth
      maxWidth="sm"
    >
      <form onSubmit={submit}>
        <DialogTitle>Site Settings</DialogTitle>
        <DialogContent
          sx={{
            display: "flex",
            flexDirection: "column",
            gap: 2,
            pt: "16px !important",
          }}
        >
          <TextField
            label="Hostname"
            value={site?.id ?? ""}
            fullWidth
            slotProps={{ input: { readOnly: true } }}
          />
          {saveError && <Alert severity="error">{saveError}</Alert>}
          <ShortcutDestination
            value={shortcut.value}
            onChange={shortcut.change}
            disabled={readOnly || saving}
          />
        </DialogContent>
        <DialogActions>
          <Button data-testid="cancel" onClick={onClose} disabled={saving}>
            Cancel
          </Button>
          <Button
            data-testid="save"
            type="submit"
            variant="contained"
            disabled={readOnly || saving}
          >
            Save
          </Button>
        </DialogActions>
      </form>
    </Dialog>
  );
}
