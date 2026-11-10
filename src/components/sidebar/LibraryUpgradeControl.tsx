import { useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { ExpandMore } from "@mui/icons-material";
import {
  Alert,
  Button,
  ButtonBase,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  Menu,
  MenuItem,
  Tooltip,
  useMediaQuery,
} from "@mui/material";
import {
  usePlatform,
  type LibraryStatus,
  type LibraryUpgrade,
} from "../../platform/Platform";
import css from "./Sidebar.module.css";

const releasesUrl = "https://github.com/reedable/xaga/releases";

function explanation(status: LibraryStatus): string {
  switch (status.readOnlyCause) {
    case "older":
      return `Library version ${status.storedVersion} needs an upgrade for Xaga ${status.appVersion}.`;
    case "newer":
      return `Library version ${status.storedVersion} requires a compatible Xaga release. This Xaga uses ${status.appVersion}.`;
    case "unversioned":
      return "This library has no version metadata. An explicit upgrade is required.";
    case "pending_upgrade":
      return `An upgrade to ${status.pendingTarget} is incomplete. Changes are already partially applied.`;
    case "pending_other_version":
      return `An upgrade to ${status.pendingTarget} is incomplete. Resume it using Xaga ${status.pendingTarget}; this Xaga uses ${status.appVersion}.`;
    case "invalid_metadata":
      return `${status.metadataError} Correct the metadata problem before reopening the library. Xaga has left it unchanged.`;
    case "lock_unavailable":
      return "This Xaga instance has no library write lock. Close other processes using this library and reopen it. No upgrade is required.";
    default:
      return "Reopen this library to establish compatibility and write access.";
  }
}

export default function LibraryUpgradeControl({
  badgeContainer,
}: {
  badgeContainer: HTMLSpanElement | null;
}) {
  const platform = usePlatform();
  const animate = useMediaQuery("(prefers-reduced-motion: no-preference)");
  const [status, setStatus] = useState<LibraryStatus | null>(null);
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);
  const [open, setOpen] = useState(false);
  const [upgrade, setUpgrade] = useState<LibraryUpgrade>({ state: "idle" });
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [restarting, setRestarting] = useState(false);
  const submitGuard = useRef(false);
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    platform
      .getLibraryStatus()
      .then((value) => {
        if (mounted.current) setStatus(value);
      })
      .catch(() => {
        if (mounted.current)
          setError(
            "Library status could not be read. Close and reopen Xaga to check the library."
          );
      });
    platform
      .getLibraryUpgrade()
      .then((value) => {
        if (mounted.current && value.state !== "idle") {
          setUpgrade(value);
          setOpen(true);
        }
      })
      .catch(() => {});
    return () => {
      mounted.current = false;
    };
  }, [platform]);

  const restart = useCallback(async () => {
    setRestarting(true);
    try {
      const message = await platform.restartAfterLibraryUpgrade();
      if (mounted.current) setError(message);
    } catch {
      if (mounted.current)
        setError(
          "The library upgrade completed, but Xaga could not restart. Close Xaga and reopen this library."
        );
    } finally {
      if (mounted.current) setRestarting(false);
    }
  }, [platform]);

  useEffect(() => {
    if (upgrade.state !== "running") return;
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try {
        const next = await platform.getLibraryUpgrade();
        if (cancelled) return;
        setUpgrade(next);
        setError(null);
        if (next.state === "running") timer = setTimeout(poll, 500);
      } catch {
        if (!cancelled) {
          setError(
            "Progress could not be read. The upgrade may still be running; keep Xaga open while it checks again."
          );
          timer = setTimeout(poll, 1000);
        }
      }
    };
    timer = setTimeout(poll, 250);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [platform, upgrade.state]);

  useEffect(() => {
    if (upgrade.state !== "succeeded") return;
    void restart();
  }, [upgrade.state, restart]);

  const start = async () => {
    if (!status || submitGuard.current) return;
    submitGuard.current = true;
    setSubmitting(true);
    setError(null);
    try {
      setUpgrade(await platform.startLibraryUpgrade(status.path));
    } catch {
      // A lost IPC response does not prove the conversion failed to start.
      setUpgrade({ state: "running", progress: null });
      setError(
        "The upgrade response was interrupted. Checking its current progress…"
      );
    } finally {
      submitGuard.current = false;
      if (mounted.current) setSubmitting(false);
    }
  };
  const cause = status?.readOnlyCause;
  const canUpgrade =
    cause === "older" || cause === "unversioned" || cause === "pending_upgrade";
  const actionable = canUpgrade || cause === "newer";
  const action =
    cause === "pending_upgrade" ? "Resume Library Upgrade" : "Upgrade Library";
  const busy = submitting || upgrade.state === "running" || restarting;
  const description = status
    ? explanation(status)
    : (error ?? "Checking library compatibility…");
  const close = () => {
    if (!busy) {
      setOpen(false);
      setError(null);
    }
  };
  const progress = upgrade.state === "running" ? upgrade.progress : null;
  const succeeded = upgrade.state === "succeeded";
  return (
    <>
      {badgeContainer &&
        createPortal(
          <>
            <Tooltip
              title={description}
              slotProps={{ transition: { timeout: animate ? undefined : 0 } }}
            >
              {actionable ? (
                <ButtonBase
                  className={css.ReadOnlyBadge}
                  aria-label="Read-only library options"
                  aria-haspopup="menu"
                  aria-expanded={!!anchor}
                  aria-controls={anchor ? "library-options" : undefined}
                  onClick={(event) => setAnchor(event.currentTarget)}
                >
                  Read-only
                  <ExpandMore className={css.BadgeCaret} />
                </ButtonBase>
              ) : (
                <span
                  className={css.ReadOnlyBadge}
                  tabIndex={0}
                  aria-label={`Read-only. ${description}`}
                >
                  Read-only
                </span>
              )}
            </Tooltip>
          </>,
          badgeContainer
        )}
      <Menu
        id="library-options"
        transitionDuration={animate ? "auto" : 0}
        anchorEl={
          (anchor?.closest("[data-library-toolbar]") as HTMLElement) ?? anchor
        }
        anchorOrigin={{ vertical: "bottom", horizontal: "right" }}
        transformOrigin={{ vertical: "top", horizontal: "left" }}
        slotProps={{ paper: { sx: { marginLeft: "0.5rem" } } }}
        open={!!anchor && !!badgeContainer}
        onClose={() => setAnchor(null)}
      >
        <MenuItem
          onClick={() => {
            setAnchor(null);
            setOpen(true);
            if (cause === "newer") {
              platform
                .openUrl(releasesUrl)
                .then(() => setOpen(false))
                .catch(() =>
                  setError(
                    "The releases page could not be opened. Try Upgrade Xaga again."
                  )
                );
            }
          }}
        >
          {cause === "newer" ? "Upgrade Xaga" : action}
        </MenuItem>
      </Menu>
      <Dialog
        open={open}
        transitionDuration={animate ? undefined : 0}
        slotProps={{
          backdrop: { transitionDuration: animate ? undefined : 0 },
        }}
        onClose={close}
        aria-labelledby="library-upgrade-title"
        maxWidth="sm"
        fullWidth
      >
        <DialogTitle id="library-upgrade-title">
          {succeeded
            ? "Library upgrade complete"
            : cause === "newer"
              ? "Upgrade Xaga"
              : action}
        </DialogTitle>
        <DialogContent>
          {cause === "newer" ? (
            <DialogContentText>{description}</DialogContentText>
          ) : (
            <>
              <DialogContentText>{description}</DialogContentText>
              <DialogContentText className={css.UpgradeParagraph}>
                This changes your library to format {status?.appVersion}. It
                cannot be undone in Xaga, and older Xaga versions may no longer
                use this library safely.
              </DialogContentText>
              <DialogContentText className={css.UpgradeParagraph}>
                Before continuing, back up the complete library and close all
                other processes using it. Restoring that complete backup is the
                way to return to the previous format. The write lock does not
                exclude existing readers.
              </DialogContentText>
              {upgrade.state === "running" && (
                <div role="status" className={css.UpgradeParagraph}>
                  {progress?.phase === "finishing"
                    ? "Finishing conversion and cache reconciliation…"
                    : progress?.phase === "converting"
                      ? "Converting library and reconciling cache…"
                      : "Preparing library upgrade…"}
                  {progress && (
                    <p>
                      {progress.converted} files converted ·{" "}
                      {progress.unchanged} unchanged · {progress.reconciled}{" "}
                      cache entries reconciled
                    </p>
                  )}
                  <p>
                    Keep Xaga open until the upgrade finishes. Changes already
                    saved cannot be rolled back by closing Xaga.
                  </p>
                </div>
              )}
              {upgrade.state === "failed" && (
                <Alert severity="error" className={css.UpgradeParagraph}>
                  {upgrade.message}
                  {upgrade.errors.length > 0 && (
                    <details>
                      <summary>Upgrade problems</summary>
                      <ul>
                        {upgrade.errors.map((message, i) => (
                          <li key={i}>{message}</li>
                        ))}
                      </ul>
                    </details>
                  )}
                </Alert>
              )}
            </>
          )}
          {error && (
            <Alert severity="error" className={css.UpgradeParagraph}>
              {error}
            </Alert>
          )}
        </DialogContent>
        <DialogActions>
          {!busy && (
            <Button onClick={close}>
              {upgrade.state === "idle" && cause !== "newer"
                ? "Cancel"
                : "Close"}
            </Button>
          )}
          {cause === "newer" ? (
            <Button
              onClick={() => {
                setError(null);
                platform
                  .openUrl(releasesUrl)
                  .then(() => setOpen(false))
                  .catch(() =>
                    setError(
                      "The releases page could not be opened. Try Upgrade Xaga again."
                    )
                  );
              }}
            >
              Upgrade Xaga
            </Button>
          ) : succeeded ? (
            <Button disabled={busy} onClick={() => void restart()}>
              Restart Xaga
            </Button>
          ) : (
            <Button disabled={busy} onClick={() => void start()}>
              {upgrade.state === "failed" ? "Retry Library Upgrade" : action}
            </Button>
          )}
        </DialogActions>
      </Dialog>
    </>
  );
}
