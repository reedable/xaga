import { LinkOff } from "@mui/icons-material";
import {
  Button,
  Dialog,
  DialogActions,
  DialogTitle,
  IconButton,
  List,
  Tooltip,
} from "@mui/material";
import { useState } from "react";
import { pinKey, type PinTarget } from "../../pins";
import MissingShortcut from "./MissingShortcut";

export default function MissingShortcutsButton({
  targets,
}: {
  targets: PinTarget[];
}) {
  const [open, setOpen] = useState(false);
  if (!targets.length) return null;
  return (
    <>
      <Tooltip title="Unavailable shortcuts">
        <IconButton
          aria-label={`Unavailable shortcuts (${targets.length})`}
          onClick={() => setOpen(true)}
          size="small"
        >
          <LinkOff fontSize="small" />
        </IconButton>
      </Tooltip>
      <Dialog
        open={open}
        onClose={() => setOpen(false)}
        aria-labelledby="missing-shortcuts-title"
      >
        <DialogTitle id="missing-shortcuts-title">
          Unavailable shortcuts
        </DialogTitle>
        <List>
          {targets.map((target) => (
            <MissingShortcut key={pinKey(target)} target={target} />
          ))}
        </List>
        <DialogActions>
          <Button onClick={() => setOpen(false)}>Close</Button>
        </DialogActions>
      </Dialog>
    </>
  );
}
