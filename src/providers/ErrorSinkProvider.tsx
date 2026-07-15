import { Alert, Snackbar } from "@mui/material";
import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { errorMessage, isAppError } from "../platform/errors";
import ErrorSinkContext, {
  type ErrorSinkContextType,
} from "./ErrorSinkContext";

/**
 * The single app-wide error surface. Every user-facing error funnels here
 * — the error sibling of the event funnel. It renders one MUI Snackbar (latest
 * error wins) showing only the user-safe `message`; the technical `detail` is
 * logged to the console, never shown.
 *
 * Sources wired here:
 *  - Explicit `pushError(e)` calls from providers/hooks (e.g. import/export).
 *  - A window `unhandledrejection` catch-all for rejected `invoke` promises
 *    that no caller handled — filtered to serialized `AppError`s so unrelated
 *    promise rejections don't surface.
 */
export default function ErrorSinkProvider({
  children,
}: {
  children: ReactNode;
}) {
  const [message, setMessage] = useState<string | null>(null);

  const pushError = useCallback((e: unknown) => {
    if (isAppError(e)) {
      // Full technical text to the console/log only — never to the user.
      console.error(`[${e.code}] ${e.detail}`);
    } else {
      console.error(e);
    }
    setMessage(errorMessage(e));
  }, []);

  // Catch-all for rejected `invoke` promises no caller handled. Only surface
  // recognized AppErrors; leave other rejections to their own handlers.
  useEffect(() => {
    const onRejection = (event: PromiseRejectionEvent) => {
      if (isAppError(event.reason)) pushError(event.reason);
    };
    window.addEventListener("unhandledrejection", onRejection);
    return () => window.removeEventListener("unhandledrejection", onRejection);
  }, [pushError]);

  const value = useMemo<ErrorSinkContextType>(() => ({ pushError }), [pushError]);

  return (
    <ErrorSinkContext.Provider value={value}>
      {children}
      <Snackbar
        open={message !== null}
        autoHideDuration={6000}
        onClose={() => setMessage(null)}
        anchorOrigin={{ vertical: "bottom", horizontal: "center" }}
      >
        <Alert
          severity="error"
          variant="filled"
          onClose={() => setMessage(null)}
        >
          {message}
        </Alert>
      </Snackbar>
    </ErrorSinkContext.Provider>
  );
}
