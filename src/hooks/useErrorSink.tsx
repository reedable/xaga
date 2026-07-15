import { useContext } from "react";
import ErrorSinkContext from "../providers/ErrorSinkContext";

/** Access the single app-wide error surface. See `ErrorSinkProvider`. */
export function useErrorSink() {
  const ctx = useContext(ErrorSinkContext);
  if (!ctx) {
    throw new Error("useErrorSink must be used within an ErrorSinkProvider");
  }
  return ctx;
}
