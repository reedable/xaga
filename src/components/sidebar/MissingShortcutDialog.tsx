import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
} from "@mui/material";
import { useContext, useId, useState } from "react";
import type { PinTarget } from "../../pins";
import { usePins } from "../../providers/PinsContext";
import { useLibraryReadOnly } from "../../hooks/useLibraryReadOnly";
import { useErrorSink } from "../../hooks/useErrorSink";
import { usePlatform } from "../../platform/Platform";
import { errorCode, errorMessage } from "../../platform/errors";
import TopicsContext from "../../providers/TopicsContext";
import FeedsContext from "../../providers/FeedsContext";
import css from "./ShortcutList.module.css";

export default function MissingShortcutDialog({
  target,
  open,
  onClose,
}: {
  target: PinTarget;
  open: boolean;
  onClose: () => void;
}) {
  const { setPin, refreshPins } = usePins();
  const platform = usePlatform();
  const readOnly = useLibraryReadOnly();
  const { pushError } = useErrorSink();
  const topics = useContext(TopicsContext);
  const feeds = useContext(FeedsContext);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string>();
  const targetName = target.type === "entry" ? "article" : target.type;
  const cannotFind = `We can’t find this ${targetName}.`;
  const titleId = useId();
  const descriptionId = useId();
  const retry = async () => {
    setBusy(true);
    setMessage(undefined);
    const signal = new AbortController().signal;
    try {
      let found = false;
      if (target.type === "site") {
        found = true;
      } else if (target.type === "topic") {
        // Direct reads preserve errors that the bulk Topic list skips.
        await platform.readTopic(signal, target.id);
        const results = await platform.listTopics(signal);
        topics?.setTopics(results);
        found = results.some((topic) => topic.id === target.id);
      } else if (target.type === "feed") {
        const results = await platform.listFeeds(signal);
        feeds?.setFeeds(results);
        found = results.some((feed) => feed.id === target.id);
      } else {
        await platform.readEntry(signal, target.id);
        await refreshPins?.();
        found = true;
      }
      if (found) onClose();
      else setMessage(cannotFind);
    } catch (error) {
      setMessage(
        errorCode(error)?.endsWith(":not-found")
          ? cannotFind
          : errorMessage(error)
      );
    } finally {
      setBusy(false);
    }
  };

  const remove = async () => {
    setBusy(true);
    try {
      await setPin(target, false);
      onClose();
    } catch (error) {
      setMessage(errorMessage(error));
      pushError(error);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog
      open={open}
      onClose={() => {
        if (!busy) onClose();
      }}
      aria-labelledby={titleId}
      aria-describedby={descriptionId}
    >
      <DialogTitle id={titleId}>Shortcut unavailable</DialogTitle>
      <DialogContent>
        <DialogContentText id={descriptionId}>
          {cannotFind} Retry to check again. Removing the shortcut leaves the
          target untouched.
        </DialogContentText>
        <DialogContentText className={css.TargetDetails}>
          {target.type}: {target.type === "site" ? target.hostname : target.id}
        </DialogContentText>
        <div role="status" aria-live="polite">
          {busy ? "Working…" : message}
        </div>
      </DialogContent>
      <DialogActions>
        <Button disabled={busy} onClick={() => onClose()}>
          Close
        </Button>
        <Button disabled={busy || readOnly} onClick={() => void remove()}>
          Remove shortcut
        </Button>
        <Button disabled={busy} onClick={() => void retry()}>
          Retry
        </Button>
      </DialogActions>
    </Dialog>
  );
}
