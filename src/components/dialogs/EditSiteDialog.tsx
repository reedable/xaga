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
import { useEffect, useState, type SubmitEvent } from "react";
import { usePins } from "../../providers/PinsContext";
import { useLibraryReadOnly } from "../../hooks/useLibraryReadOnly";
import { useErrorSink } from "../../hooks/useErrorSink";
import type { Site } from "../../types";

export default function EditSiteDialog({
  site,
  onClose,
}: {
  site: Site | null;
  onClose: () => void;
}) {
  const { pins, setPin } = usePins();
  const readOnly = useLibraryReadOnly();
  const { pushError } = useErrorSink();
  const [pinned, setPinned] = useState(false);
  const [saving, setSaving] = useState(false);
  useEffect(() => {
    setPinned(
      !!site &&
        pins.some(
          (pin) => pin.type === "site" && pin.hostname === site.hostname
        )
    );
  }, [site, pins]);
  const submit = async (event: SubmitEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!site || readOnly || saving) return;
    setSaving(true);
    try {
      await setPin({ type: "site", hostname: site.hostname }, pinned);
      onClose();
    } catch (error) {
      pushError(error);
    } finally {
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
            value={site?.hostname ?? ""}
            fullWidth
            slotProps={{ input: { readOnly: true } }}
          />
          <FormControlLabel
            label="Add to Shortcuts"
            control={
              <Checkbox
                checked={pinned}
                onChange={(event) => setPinned(event.target.checked)}
                disabled={readOnly || saving}
              />
            }
          />
        </DialogContent>
        <DialogActions>
          <Button onClick={onClose} disabled={saving}>
            Cancel
          </Button>
          <Button
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
