import { Box, Button, Chip, TextField, Typography } from "@mui/material";
import { Check } from "@mui/icons-material";
import { useEffect, useRef, useState } from "react";
import type { EntryTagChange, Tag, TagMerge } from "../../domain";
import { usePlatform } from "../../platform/Platform";
import { useErrorSink } from "../../hooks/useErrorSink";
import { useLibraryReadOnly } from "../../hooks/useLibraryReadOnly";
import { tagNameComparison, resolveMergedTagIds, retainTagMerges } from "../../utils/tags";
import css from "./EntryTags.module.css";

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
  const [deleting, setDeleting] = useState<string[]>([]);
  const [assigned, setAssigned] = useState<Set<string>>(new Set());
  const [loaded, setLoaded] = useState(false);
  const [failed, setFailed] = useState(false);
  const [query, setQuery] = useState("");
  const [busy, setBusy] = useState(false);
  const mergeHistory = useRef<TagMerge[]>([]);
  const assignmentRequest = useRef(0);
  const pending = useRef(false);
  const live = useRef(false);

  useEffect(() => {
    live.current = true;
    const controller = new AbortController();
    const initialChanges = new Map<string, EntryTagChange>();
    let loading = true;
    let receivedSnapshot = false;
    let assignmentsRefreshed = false;
    mergeHistory.current = [];
    const refreshAssignments = async () => {
      const request = ++assignmentRequest.current;
      try {
        const ids = await platform.readEntryTags(controller.signal, entryId);
        if (!controller.signal.aborted && request === assignmentRequest.current) {
          assignmentsRefreshed = true;
          setAssigned(new Set(resolveMergedTagIds(ids, mergeHistory.current)));
        }
      } catch (error) { if (!controller.signal.aborted) pushError(error); }
    };
    const cleanups: (() => void)[] = [];

    const retain = (cleanup: () => void) => {
      if (controller.signal.aborted) cleanup();
      else cleanups.push(cleanup);
    };

    const addDefinition = (tag: Tag) => {
      if (!controller.signal.aborted)
        setTags((current) => [...current.filter((item) => item.id !== tag.id), tag]);
    };

    void (async () => {
      try {
        retain(await platform.onTagCreated(addDefinition));
        retain(await platform.onTagsChanged((snapshot) => {
          if (!controller.signal.aborted) {
            receivedSnapshot = true; setTags(snapshot.tags); setDeleting(snapshot.deleting);
            mergeHistory.current = retainTagMerges(mergeHistory.current, snapshot.merging);
            setAssigned((ids) => new Set(resolveMergedTagIds(ids, mergeHistory.current)));
            if (mergeHistory.current.length) void refreshAssignments();
          }
        }));

        retain(
          await platform.onEntryTagsChanged((change) => {
            if (change.entryId !== entryId || controller.signal.aborted) return;
            if (mergeHistory.current.length) { void refreshAssignments(); return; }
            if (loading) initialChanges.set(change.tagId, change);
            setAssigned((current) => applyChange(current, change));
          }),
        );

        const [definitions, ids] = await Promise.all([
          platform.readTagStatus(controller.signal),
          platform.readEntryTags(controller.signal, entryId),
        ]);

        if (controller.signal.aborted) return;

        if (!receivedSnapshot) { setDeleting(definitions.deleting); setTags((current) => [
          ...definitions.tags,
          ...current.filter((tag) => !definitions.tags.some((item) => item.id === tag.id)),
        ]); }

        mergeHistory.current = retainTagMerges(mergeHistory.current, definitions.merging);
        if (!assignmentsRefreshed) setAssigned(new Set(resolveMergedTagIds([...initialChanges.values()].reduce(applyChange, new Set(ids)), mergeHistory.current)));
        loading = false;
        initialChanges.clear();
        setLoaded(true);
      } catch (error) {
        if (!controller.signal.aborted) {
          setFailed(true);
          pushError(error);
        }
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
      const chosen = tag ?? (await platform.createTag(query.trim()));
      // Creating the definition does not depend on the Inspector staying open.
      const change = await platform.setEntryTag(
        entryId,
        chosen.id,
        tag ? !assigned.has(tag.id) : true,
      );

      if (!live.current) return;
      if (!mergeHistory.current.some((merge) => [merge.sourceId, merge.destinationId].includes(chosen.id))) {
        setTags((current) => [...current.filter((item) => item.id !== chosen.id), chosen]);
      }
      if (mergeHistory.current.length) {
        const request = ++assignmentRequest.current;
        const ids = await platform.readEntryTags(new AbortController().signal, entryId);
        if (live.current && request === assignmentRequest.current) setAssigned(new Set(resolveMergedTagIds(ids, mergeHistory.current)));
      } else setAssigned((current) => applyChange(current, change));
      if (!tag) setQuery("");
    } catch (error) {
      pushError(error);
    } finally {
      pending.current = false;
      if (live.current) setBusy(false);
    }
  };

  const comparison = tagNameComparison(query);

  const matches = tags
    .filter((tag) => tagNameComparison(tag.name).includes(comparison))
    .sort((a, b) => a.name.localeCompare(b.name));

  const canCreate = comparison && !tags.some((tag) => tagNameComparison(tag.name) === comparison);

  if (failed)
    return (
      <Box className={css.EntryTags} aria-busy={busy}>
        <Typography>Tags couldn’t be loaded.</Typography>
      </Box>
    );

  if (!loaded)
    return (
      <Box className={css.EntryTags} aria-busy={busy}>
        <Typography role="status">Loading tags…</Typography>
      </Box>
    );

  return (
    <Box className={css.EntryTags} aria-busy={busy}>
      <TextField
        className={css.FilterInput}
        placeholder="Find or create a tag"
        size="small"
        value={query}
        variant="standard"
        onChange={(event) => setQuery(event.target.value)}
      />

      <Box className={css.TagGroup} role="group" aria-label="Entry tags">
        {matches.map((tag) => (
          <Chip
            key={tag.id}
            label={tag.name}
            component="button"
            type="button"
            clickable
            size="small"
            aria-pressed={assigned.has(tag.id)}
            color={assigned.has(tag.id) ? "primary" : "default"}
            variant={assigned.has(tag.id) ? "filled" : "outlined"}
            icon={assigned.has(tag.id) ? <Check /> : undefined}
            disabled={readOnly || busy || deleting.includes(tag.id)}
            onClick={() => void mutate(tag)}
          />
        ))}
      </Box>

      {!matches.length && (
        <Typography variant="body2">
          {tags.length ? "No matching tags." : "No tags yet."}
        </Typography>
      )}

      {canCreate && (
        <Button disabled={readOnly || busy} onClick={() => void mutate()}>
          Create “{query.trim()}”
        </Button>
      )}
    </Box>
  );
}
