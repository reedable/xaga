import { createContext } from "react";

export type ErrorSinkContextType = {
  /**
   * Surface an error to the user on the single app-wide error surface. Accepts
   * any caught value; a serialized `AppError` shows its user-safe `message`,
   * anything else shows a generic message. The technical `detail` is logged,
   * never shown.
   */
  pushError: (e: unknown) => void;
};

const ErrorSinkContext = createContext<ErrorSinkContextType | null>(null);

export default ErrorSinkContext;
