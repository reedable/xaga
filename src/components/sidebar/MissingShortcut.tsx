import { WarningAmberOutlined } from "@mui/icons-material";
import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  ListItem,
  ListItemButton,
  ListItemIcon,
  ListItemText,
} from "@mui/material";
import { useContext, useId, useState } from "react";
import type { PinTarget } from "../../pins";
import { usePins } from "../../providers/PinsContext";
import { useLibraryReadOnly } from "../../hooks/useLibraryReadOnly";
import { useErrorSink } from "../../hooks/useErrorSink";
import { usePlatform } from "../../platform/Platform";
import { errorCode, errorMessage } from "../../platform/errors";
import SitesContext from "../../providers/SitesContext";
import TopicsContext from "../../providers/TopicsContext";
import FeedsContext from "../../providers/FeedsContext";
import css from "./ShortcutList.module.css";
import itemCss from "./SidebarItem.module.css";

export default function MissingShortcut({ target }: { target: PinTarget }) {
  const { setPin, refreshPins } = usePins();
  const platform = usePlatform();
  const readOnly = useLibraryReadOnly();
  const { pushError } = useErrorSink();
  const topics = useContext(TopicsContext);
  const sites = useContext(SitesContext);
  const feeds = useContext(FeedsContext);
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string>();
  const targetName = target.type === "entry" ? "article" : target.type;
  const cannotFind = `We can’t find this ${targetName}.`;
  const titleId = useId();
  const descriptionId = useId();
  const pending =
    target.type === "site"
      ? sites?.hasLoaded === false
      : target.type === "topic"
        ? topics?.hasLoaded === false
        : target.type === "feed" && feeds?.hasLoaded === false;

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
      if (found) setOpen(false);
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
      setOpen(false);
    } catch (error) {
      setMessage(errorMessage(error));
      pushError(error);
    } finally {
      setBusy(false);
    }
  };

  if (pending) {
    return (
      <ListItem
        aria-label="Loading shortcut"
        aria-busy="true"
        className={css.SkeletonRow}
      >
        <span aria-hidden="true" className={css.SkeletonIcon} />
        <span aria-hidden="true" className={css.SkeletonTitle} />
      </ListItem>
    );
  }
  return (
    <>
      <ListItem disablePadding>
        <ListItemButton onClick={() => setOpen(true)}>
          <ListItemIcon className={itemCss.ListItemIcon}>
            <WarningAmberOutlined className={itemCss.Icon} />
          </ListItemIcon>
          <ListItemText
            primary="Shortcut unavailable"
            slotProps={{ primary: { className: itemCss.ListItemTextPrimary } }}
          />
        </ListItemButton>
      </ListItem>
      <Dialog
        open={open}
        onClose={() => {
          if (!busy) setOpen(false);
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
            {target.type}:{" "}
            {target.type === "site" ? target.hostname : target.id}
          </DialogContentText>
          <div role="status" aria-live="polite">
            {busy ? "Working…" : message}
          </div>
        </DialogContent>
        <DialogActions>
          <Button disabled={busy} onClick={() => setOpen(false)}>
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
    </>
  );
}
