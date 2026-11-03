import { MoreVert } from "@mui/icons-material";
import { Button, Dialog, DialogActions, DialogContent, DialogTitle, IconButton, Menu, MenuItem, TextField, Tooltip, useMediaQuery } from "@mui/material";
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
  const { renaming = [] } = useTags();
  const { pushError } = useErrorSink();
  const trigger = useRef<HTMLButtonElement>(null);
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);
  const [open, setOpen] = useState(false);
  const [name, setName] = useState(tag.name);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const pending = useRef(false);
  const busy = renaming.includes(tag.id);
  const close = () => { if (!pending.current) setOpen(false); };
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
      }}>{busy ? "Renaming…" : "Rename"}</MenuItem>
    </Menu>
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
