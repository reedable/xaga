import { Alert, Snackbar } from "@mui/material";
import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { errorMessage, isAppError } from "../platform/errors";
import ErrorSinkContext, {
  type ErrorSinkContextType,
} from "./ErrorSinkContext";

/**
 * Shows user-facing error: one MUI Snackbar, latest error wins.
 * Displays the user-safe `message`; `detail` goes to the console.
 *
 * Errors arrive two ways:
 *  - `pushError(e)` calls.
 *  - Unhandled promise rejections that carry an `AppError`.
 */
export default function ErrorSinkProvider({
  children,
}: {
  children: ReactNode;
}) {
  const [message, setMessage] = useState<string | null>(null);

  const pushError = useCallback((e: unknown) => {
    // Aborts (navigation/unmount cancelling an in-flight request) are not
    // failures — never surface them. Every catch can forward safely.
    if (e instanceof DOMException && e.name === "AbortError") return;
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

  const value = useMemo<ErrorSinkContextType>(
    () => ({ pushError }),
    [pushError]
  );

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
