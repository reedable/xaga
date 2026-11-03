import { MoreVert } from "@mui/icons-material";
import { Button, Dialog, DialogActions, DialogContent, DialogContentText, DialogTitle, IconButton, Menu, MenuItem, TextField, Tooltip, useMediaQuery } from "@mui/material";
import { useRef, useState } from "react";
import type { Tag } from "../domain";
import { usePlatform } from "../platform/Platform";
import { useLibraryReadOnly } from "../hooks/useLibraryReadOnly";
import { useErrorSink } from "../hooks/useErrorSink";
import { useTags } from "../providers/TagsContext";
import { isAppError } from "../platform/errors";
import css from "./TagActions.module.css";

export function TagActions({ tag }: { tag: Tag }) {
  const platform = usePlatform();
  const animate = useMediaQuery("(prefers-reduced-motion: no-preference)");
  const readOnly = useLibraryReadOnly();
  const { renaming = [], deleting = [] } = useTags();
  const { pushError } = useErrorSink();
  const trigger = useRef<HTMLButtonElement>(null);
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);
  const [open, setOpen] = useState(false);
  const [deletingDialog, setDeletingDialog] = useState(false);
  const [name, setName] = useState(tag.name);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const pending = useRef(false);
  const isDeleting = deleting.includes(tag.id);
  const busy = renaming.includes(tag.id) || isDeleting;
  const close = () => { if (!pending.current) { setOpen(false); setDeletingDialog(false); } };
  const submit = async (event: React.SubmitEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (pending.current || readOnly || busy || !name.trim()) return;
    pending.current = true; setSaving(true); setError("");
    try {
      await platform.renameTag(tag.id, name.trim());
      setOpen(false);
    } catch (cause) {
      if (isAppError(cause) && ["tags:collision", "tags:invalid-name", "tags:busy"].includes(cause.code)) {
        setError(cause.message);
      } else { pushError(cause); }
    } finally { pending.current = false; setSaving(false); }
  };
  const confirmDelete = async () => {
    if (pending.current || readOnly || busy) return;
    pending.current = true; setSaving(true);
    try { await platform.deleteTag(tag.id); setDeletingDialog(false); }
    catch (cause) { pushError(cause); }
    finally { pending.current = false; setSaving(false); }
  };
  return <>
    <Tooltip title={`Actions for ${tag.name}`} slotProps={{ transition: { timeout: animate ? undefined : 0 } }}><IconButton ref={trigger} size="small" className={css.Trigger}
      data-tag-action
      aria-label={`Actions for ${tag.name}`} aria-haspopup="menu" aria-expanded={!!anchor}
      onMouseDown={(event) => event.stopPropagation()}
      onKeyDown={(event) => event.stopPropagation()}
      onClick={(event) => { event.stopPropagation(); setAnchor(event.currentTarget); }}>
      <MoreVert fontSize="small" />
    </IconButton></Tooltip>
    <Menu transitionDuration={animate ? "auto" : 0} anchorEl={anchor} open={!!anchor} onClose={() => setAnchor(null)}
      onClick={(event) => event.stopPropagation()}>
      <MenuItem disabled={readOnly || busy} onClick={() => {
        setAnchor(null); setName(tag.name); setError(""); setOpen(true);
      }}>{renaming.includes(tag.id) ? `Renaming “${tag.name}”…` : `Rename “${tag.name}”`}</MenuItem>
      <MenuItem disabled={readOnly || busy} onClick={() => { setAnchor(null); setDeletingDialog(true); }}>
        {isDeleting ? `Deleting “${tag.name}”…` : `Delete “${tag.name}”`}
      </MenuItem>
    </Menu>
    <Dialog transitionDuration={animate ? undefined : 0} open={deletingDialog} onClose={close} disableRestoreFocus
      slotProps={{ transition: { onExited: () => trigger.current?.focus() } }}
      onClick={(event) => event.stopPropagation()} aria-labelledby={`delete-${tag.id}`} aria-describedby={`delete-description-${tag.id}`}>
      <DialogTitle id={`delete-${tag.id}`}>Delete tag</DialogTitle>
      <DialogContent><DialogContentText id={`delete-description-${tag.id}`}>
        Delete “{tag.name}” and its article assignments? Articles will remain intact.
      </DialogContentText></DialogContent>
      <DialogActions>
        <Button autoFocus disabled={saving} onClick={close}>Cancel</Button>
        <Button color="error" disabled={saving || busy || readOnly} onClick={() => void confirmDelete()}>Delete</Button>
      </DialogActions>
    </Dialog>
    <Dialog transitionDuration={animate ? undefined : 0} open={open} onClose={close} disableRestoreFocus
      slotProps={{ transition: { onExited: () => trigger.current?.focus() } }}
      onClick={(event) => event.stopPropagation()} aria-labelledby={`rename-${tag.id}`}>
      <form onSubmit={submit}>
        <DialogTitle id={`rename-${tag.id}`}>Rename tag</DialogTitle>
        <DialogContent>
          <TextField autoFocus margin="dense" label="Tag name" fullWidth
            value={name} disabled={saving} error={!!error} helperText={error || undefined}
            onChange={(event) => { setName(event.target.value); setError(""); }} />
        </DialogContent>
        <DialogActions>
          <Button disabled={saving} onClick={close}>Cancel</Button>
          <Button type="submit" disabled={saving || busy || readOnly || !name.trim()}>Rename</Button>
        </DialogActions>
      </form>
    </Dialog>
  </>;
}
