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
 * Shows user-facing error: one MUI Snackbar, latest error wins.
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
  const [diagnosticId, setDiagnosticId] = useState<string | null>(null);
  const [detailsId, setDetailsId] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [severity, setSeverity] = useState<"error" | "warning">("error");
  const reportedIcons = useRef(new Set<string>());

  const pushError = useCallback((e: unknown) => {
    // Aborts (navigation/unmount cancelling an in-flight request) are not
    // failures — never surface them. Every catch can forward safely.
    if (isAbortError(e)) return;
    if (e instanceof FaviconLoadError) {
      if (reportedIcons.current.has(e.articleUrl)) return;
      reportedIcons.current.add(e.articleUrl);
      setDiagnosticId(e.diagnosticId);
      setSeverity("warning");
      setMessage(e.message);
      return;
    }
    setSeverity("error");
    setDiagnosticId(
      e instanceof ArticleLoadError
        ? e.diagnosticId || null
        : isAppError(e) ? e.diagnosticId ?? null : null
    );
    if (e instanceof ArticleLoadError) {
      setMessage(
        `Article loading failed during ${e.step.replaceAll("_", " ")}. ${e.message}`
      );
      return;
    }
    if (isAppError(e)) {
      // Full technical text to the console/log only — never to the user.
      console.error(`[${e.code}] ${e.detail}`);
    } else {
      console.error(e);
    }
    setMessage(errorMessage(e));
  }, []);

  useEffect(() => {
    if (!subscribeErrors) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    subscribeErrors((error) => {
      if (!disposed) pushError(error);
    }).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    }).catch((error) => {
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
        open={message !== null}
        autoHideDuration={diagnosticId ? null : 6000}
        onClose={() => setMessage(null)}
        anchorOrigin={{ vertical: "bottom", horizontal: "center" }}
      >
        <Alert
          severity={severity}
          variant="filled"
          onClose={() => setMessage(null)}
        >
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
