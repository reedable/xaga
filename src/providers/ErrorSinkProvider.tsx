import { Alert, Button, Snackbar } from "@mui/material";
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import ArticleDiagnosticsDialog from "../components/dialogs/ArticleDiagnosticsDialog";
import { errorMessage, isAppError } from "../platform/errors";
import { isAbortError } from "../types";
import ArticleLoadError from "../utils/ArticleLoadError";
import FaviconLoadError from "../utils/FaviconLoadError";
import ErrorSinkContext, {
  type ErrorSinkContextType,
} from "./ErrorSinkContext";

/**
 * Shows user-facing error: one MUI Snackbar, queued in arrival order.
 * Displays the user-safe `message`; `detail` goes to the console.
 *
 * Errors arrive through:
 *  - The platform error subscription.
 *  - `pushError(e)` calls.
 *  - Unhandled promise rejections that carry an `AppError`.
 */
export default function ErrorSinkProvider({
  children,
  subscribeErrors,
}: {
  children: ReactNode;
  subscribeErrors?: (callback: (error: unknown) => void) => Promise<() => void>;
}) {
  const [notifications, setNotifications] = useState<
    {
      id: number;
      message: string;
      severity: "error" | "warning";
      diagnosticId: string | null;
    }[]
  >([]);
  const [detailsId, setDetailsId] = useState<string | null>(null);
  const nextId = useRef(0);
  const current = notifications[0];
  const diagnosticId = current?.diagnosticId;
  const message = current?.message;
  const severity = current?.severity;
  const dismiss = () => {
    if (detailsId) return;
    setNotifications((queue) =>
      queue.filter((item) => item.id !== current?.id)
    );
  };
  const reportedIcons = useRef(new Set<string>());

  const pushError = useCallback((e: unknown) => {
    // Aborts (navigation/unmount cancelling an in-flight request) are not
    // failures — never surface them. Every catch can forward safely.
    if (isAbortError(e)) return;
    const favicon = e instanceof FaviconLoadError;
    if (favicon) {
      if (reportedIcons.current.has(e.articleUrl)) return;
      reportedIcons.current.add(e.articleUrl);
    }
    const diagnosticId =
      e instanceof ArticleLoadError || favicon
        ? e.diagnosticId || null
        : isAppError(e)
          ? (e.diagnosticId ?? null)
          : null;
    if (!favicon && !(e instanceof ArticleLoadError)) {
      if (isAppError(e)) console.error(`[${e.code}] ${e.detail}`);
      else console.error(e);
    }
    const message =
      e instanceof ArticleLoadError
        ? `Article loading failed during ${e.step.replaceAll("_", " ")}. ${e.message}`
        : favicon
          ? e.message
          : errorMessage(e);
    const notification = {
      id: nextId.current++,
      message,
      diagnosticId,
      severity: favicon ? ("warning" as const) : ("error" as const),
    };
    setNotifications((queue) => [...queue, notification]);
  }, []);

  useEffect(() => {
    if (!subscribeErrors) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    subscribeErrors((error) => {
      if (!disposed) pushError(error);
    })
      .then((stop) => {
        if (disposed) stop();
        else unlisten = stop;
      })
      .catch((error) => {
        if (!disposed) pushError(error);
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [subscribeErrors, pushError]);

  // Catch-all for rejected `invoke` promises no caller handled. Only surface
  // recognized AppErrors; leave other rejections to their own handlers.
  useEffect(() => {
    const onRejection = (event: PromiseRejectionEvent) => {
      if (isAppError(event.reason)) pushError(event.reason);
    };
    window.addEventListener("unhandledrejection", onRejection);
    return () => window.removeEventListener("unhandledrejection", onRejection);
  }, [pushError]);

  const value = useMemo<ErrorSinkContextType>(
    () => ({ pushError }),
    [pushError]
  );

  return (
    <ErrorSinkContext.Provider value={value}>
      {children}
      <Snackbar
        key={current?.id ?? "empty"}
        open={!!current}
        autoHideDuration={diagnosticId ? null : 6000}
        onClose={(_, reason) => {
          if (reason !== "clickaway") dismiss();
        }}
        anchorOrigin={{ vertical: "bottom", horizontal: "center" }}
      >
        <Alert severity={severity} variant="filled" onClose={dismiss}>
          {message}
          {diagnosticId && (
            <Button
              color="inherit"
              size="small"
              onClick={() => setDetailsId(diagnosticId)}
            >
              Details
            </Button>
          )}
        </Alert>
      </Snackbar>
      {detailsId && (
        <ArticleDiagnosticsDialog
          key={detailsId}
          open
          diagnosticId={detailsId}
          onClose={() => setDetailsId(null)}
        />
      )}
    </ErrorSinkContext.Provider>
  );
}
