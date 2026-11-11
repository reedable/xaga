import { useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import {
  Alert,
  Button,
  ButtonBase,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
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
      return `A previous upgrade was interrupted. Xaga can resume it and upgrade this library to ${status.appVersion}.`;
    case "pending_other_version":
      return `A newer Xaga version started upgrading this library to ${status.pendingTarget}. Update Xaga to finish the upgrade.`;
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
  const [open, setOpen] = useState(false);
  const [upgrade, setUpgrade] = useState<LibraryUpgrade>({ state: "idle" });
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [restarting, setRestarting] = useState(false);
  const submitGuard = useRef(false);
  const mounted = useRef(true);
  const continued = useRef(false);
  useEffect(() => {
    mounted.current = true;
    platform
      .getLibraryStatus()
      .then((value) => {
        if (mounted.current) {
          setStatus(value);
          if (value.readOnlyCause && !continued.current) setOpen(true);
        }
      })
      .catch(() => {
        if (mounted.current) {
          setOpen(true);
          setError(
            "Library status could not be read. Close and reopen Xaga to check the library."
          );
        }
      });
    platform
      .getLibraryUpgrade()
      .then((value) => {
        if (mounted.current && !continued.current && value.state !== "idle") {
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
  const needsNewApp = cause === "newer" || cause === "pending_other_version";
  const mandatory = status?.layoutUpgradeRequired ?? true;
  const busy = submitting || upgrade.state === "running" || restarting;
  const description = status
    ? explanation(status)
    : (error ?? "Checking library compatibility…");
  const close = async () => {
    if (busy || mandatory) return;
    try {
      await platform.continueLibraryReadOnly();
      continued.current = true;
      setOpen(false);
      setError(null);
    } catch {
      setError(
        "Read-only startup could not continue. Try again or reopen Xaga."
      );
    }
  };
  const progress = upgrade.state === "running" ? upgrade.progress : null;
  const succeeded = upgrade.state === "succeeded";
  return (
    <>
      {badgeContainer &&
        createPortal(
          <ButtonBase
            data-testid="library-options-trigger"
            className={css.ReadOnlyBadge}
            data-read-only-cause={cause}
            aria-haspopup="dialog"
            aria-label="Read-only library options"
            onClick={() => {
              setOpen(true);
              setStatus(null);
              platform
                .getLibraryStatus()
                .then(setStatus)
                .catch(() => {
                  setError(
                    "Library status could not be read. Close and reopen Xaga to check the library."
                  );
                });
              platform
                .getLibraryUpgrade()
                .then(setUpgrade)
                .catch(() => {});
            }}
          >
            Read-only
          </ButtonBase>,
          badgeContainer
        )}
      <Dialog
        open={open}
        transitionDuration={animate ? undefined : 0}
        slotProps={{
          backdrop: { transitionDuration: animate ? undefined : 0 },
          paper: {
            ...{
              "data-upgrade-state": upgrade.state,
              "data-library-version": status?.storedVersion ?? "unversioned",
              "data-target-version": status?.appVersion,
              "data-read-only-cause": cause,
            },
          },
        }}
        onClose={() => void close()}
        aria-labelledby="library-upgrade-title"
        aria-describedby="library-upgrade-description"
        maxWidth="sm"
        fullWidth
      >
        <DialogTitle id="library-upgrade-title">
          {succeeded
            ? "Library upgrade complete"
            : upgrade.state === "running"
              ? "Upgrading library"
              : needsNewApp
                ? "Upgrade Xaga"
                : canUpgrade && status
                  ? `Upgrade Library ${status.storedVersion ? `from ${status.storedVersion} ` : ""}to ${status.appVersion}`
                  : "Read-only library"}
        </DialogTitle>
        <DialogContent>
          <div
            id="library-upgrade-description"
            className={css.UpgradeDescription}
          >
            {succeeded ? (
              <DialogContentText role="status">
                {restarting
                  ? "Restarting Xaga to reopen this library…"
                  : "Conversion is complete. Restart Xaga to reopen this library."}
              </DialogContentText>
            ) : upgrade.state === "running" ? (
              <div role="status">
                {progress?.phase === "finishing"
                  ? "Finishing conversion and cache reconciliation…"
                  : progress?.phase === "converting"
                    ? "Converting library and reconciling cache…"
                    : "Preparing library upgrade…"}
                {progress && (
                  <p>
                    {progress.converted} files converted · {progress.unchanged}{" "}
                    unchanged · {progress.reconciled} cache entries reconciled
                  </p>
                )}
                <p>Keep Xaga open until the upgrade finishes.</p>
              </div>
            ) : (
              <>
                {(cause !== "older" || !canUpgrade) && (
                  <DialogContentText>{description}</DialogContentText>
                )}
                {mandatory && (
                  <DialogContentText>
                    This library must finish its directory layout upgrade before
                    it can be opened. You can upgrade it, choose another
                    library, or quit.
                  </DialogContentText>
                )}
                {canUpgrade && (
                  <>
                    <DialogContentText>
                      Older versions of Xaga will not be able to edit this
                      library after the upgrade.
                    </DialogContentText>
                    <DialogContentText>
                      Back up the entire library folder and close other Xaga
                      windows before upgrading. To undo the upgrade, you must
                      restore that backup.
                    </DialogContentText>
                  </>
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
          </div>
          {(upgrade.state === "failed" || upgrade.state === "succeeded") &&
            upgrade.read_error && (
              <Alert severity="warning" className={css.UpgradeParagraph}>
                {upgrade.read_error}
              </Alert>
            )}
          {error && (
            <Alert severity="error" className={css.UpgradeParagraph}>
              {error}
            </Alert>
          )}
        </DialogContent>
        <DialogActions>
          {!busy && mandatory && (
            <>
              <Button
                disabled={succeeded}
                data-testid="choose-library"
                onClick={async () => {
                  try {
                    const path = await platform.openFileDialog(true);
                    if (path) await platform.setLibrary(path);
                  } catch {
                    setError("The library could not be switched. Try again.");
                  }
                }}
              >
                Choose another Library
              </Button>
              <Button
                data-testid="quit-xaga"
                onClick={() => void platform.quitApp()}
              >
                Quit
              </Button>
            </>
          )}
          {!busy && !mandatory && (
            <Button
              data-testid="continue-read-only"
              onClick={() => void close()}
            >
              Continue read-only
            </Button>
          )}
          {needsNewApp ? (
            <Button
              data-testid="upgrade-xaga"
              onClick={() => {
                setError(null);
                platform
                  .openUrl(releasesUrl)
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
            <Button
              data-testid="restart-xaga"
              disabled={busy}
              onClick={() => void restart()}
            >
              Restart Xaga
            </Button>
          ) : canUpgrade ? (
            <Button
              data-testid={
                upgrade.state === "failed"
                  ? "retry-library-upgrade"
                  : "upgrade-library"
              }
              disabled={busy}
              onClick={() => void start()}
            >
              {upgrade.state === "failed" ? "Retry Library Upgrade" : "Upgrade"}
            </Button>
          ) : null}
        </DialogActions>
      </Dialog>
    </>
  );
}
