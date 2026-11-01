import { Box, Button, Chip, TextField, Typography } from "@mui/material";
import { Check } from "@mui/icons-material";
import { useEffect, useRef, useState } from "react";
import type { EntryTagChange, Tag } from "../../domain";
import { usePlatform } from "../../platform/Platform";
import { useErrorSink } from "../../hooks/useErrorSink";
import { useLibraryReadOnly } from "../../hooks/useLibraryReadOnly";

// Match Rust's per-character Unicode lowercase conversion for tag identity.
function comparisonValue(name: string) {
  return Array.from(name.trim(), (character) => character.toLowerCase()).join("");
}

function applyChange(ids: Set<string>, change: EntryTagChange) {
  const next = new Set(ids);
  if (change.assigned) next.add(change.tagId);
  else next.delete(change.tagId);
  return next;
}

export function EntryTags({ entryId }: { entryId: string }) {
  const platform = usePlatform();
  const { pushError } = useErrorSink();
  const readOnly = useLibraryReadOnly();
  const [tags, setTags] = useState<Tag[]>([]);
  const [assigned, setAssigned] = useState<Set<string>>(new Set());
  const [loaded, setLoaded] = useState(false);
  const [failed, setFailed] = useState(false);
  const [query, setQuery] = useState("");
  const [busy, setBusy] = useState(false);
  const pending = useRef(false);
  const live = useRef(false);

  useEffect(() => {
    live.current = true;
    const controller = new AbortController();
    const initialChanges = new Map<string, EntryTagChange>();
    let loading = true;
    const cleanups: (() => void)[] = [];
    const retain = (cleanup: () => void) => {
      if (controller.signal.aborted) cleanup();
      else cleanups.push(cleanup);
    };
    const addDefinition = (tag: Tag) => {
      if (!controller.signal.aborted) setTags((current) =>
        [...current.filter((item) => item.id !== tag.id), tag]);
    };
    void (async () => {
      try {
        retain(await platform.onTagCreated(addDefinition));
        retain(await platform.onEntryTagsChanged((change) => {
          if (change.entryId !== entryId || controller.signal.aborted) return;
          if (loading) initialChanges.set(change.tagId, change);
          setAssigned((current) => applyChange(current, change));
        }));
        const [definitions, ids] = await Promise.all([
          platform.listTags(controller.signal),
          platform.readEntryTags(controller.signal, entryId),
        ]);
        if (controller.signal.aborted) return;
        setTags((current) => [...definitions, ...current.filter((tag) =>
          !definitions.some((item) => item.id === tag.id))]);
        setAssigned([...initialChanges.values()].reduce(applyChange, new Set(ids)));
        loading = false;
        initialChanges.clear();
        setLoaded(true);
      } catch (error) {
        if (!controller.signal.aborted) { setFailed(true); pushError(error); }
      }
    })();
    return () => {
      live.current = false;
      controller.abort();
      cleanups.forEach((cleanup) => cleanup());
    };
  }, [entryId, platform, pushError]);

  const mutate = async (tag?: Tag) => {
    if (readOnly || pending.current || !loaded) return;
    pending.current = true;
    setBusy(true);
    try {
      const chosen = tag ?? await platform.createTag(query.trim());
      // Creating the definition does not depend on the Inspector staying open.
      const change = await platform.setEntryTag(entryId, chosen.id,
        tag ? !assigned.has(tag.id) : true);
      if (!live.current) return;
      setTags((current) => [...current.filter((item) => item.id !== chosen.id), chosen]);
      setAssigned((current) => applyChange(current, change));
      if (!tag) setQuery("");
    } catch (error) { pushError(error); }
    finally {
      pending.current = false;
      if (live.current) setBusy(false);
    }
  };
  const comparison = comparisonValue(query);
  const matches = tags.filter((tag) => comparisonValue(tag.name).includes(comparison))
    .sort((a, b) => a.name.localeCompare(b.name));
  const canCreate = comparison && !tags.some((tag) => comparisonValue(tag.name) === comparison);
  if (failed) return <Typography>Tags couldn’t be loaded.</Typography>;
  if (!loaded) return <Typography role="status">Loading tags…</Typography>;
  return <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }} aria-busy={busy}>
    <TextField label="Find or create a tag" size="small" value={query}
      onChange={(event) => setQuery(event.target.value)} />
    <Box sx={{ display: "flex", flexWrap: "wrap", gap: 1 }} role="group" aria-label="Entry tags">
      {matches.map((tag) => <Chip key={tag.id} label={tag.name}
        component="button" type="button" clickable
        aria-pressed={assigned.has(tag.id)}
        color={assigned.has(tag.id) ? "primary" : "default"}
        variant={assigned.has(tag.id) ? "filled" : "outlined"}
        icon={assigned.has(tag.id) ? <Check /> : undefined}
        disabled={readOnly || busy} onClick={() => void mutate(tag)} />)}
    </Box>
    {!matches.length && <Typography variant="body2">{tags.length ? "No matching tags." : "No tags yet."}</Typography>}
    {canCreate && <Button disabled={readOnly || busy} onClick={() => void mutate()}>
      Create “{query.trim()}”
    </Button>}
  </Box>;
}
