import { errorMessage } from "../platform/errors";
import type { ArticleDiagnosticStep } from "../types";

export default class ArticleLoadError extends Error {
  readonly diagnosticId: string;
  readonly step: ArticleDiagnosticStep;

  constructor(
    diagnosticId: string,
    step: ArticleDiagnosticStep,
    original: unknown
  ) {
    super(errorMessage(original), { cause: original });
    this.diagnosticId = diagnosticId;
    this.step = step;
  }
}
