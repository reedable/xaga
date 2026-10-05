/** A non-fatal icon failure, separate from article-loading errors. */
export default class FaviconLoadError extends Error {
  readonly articleUrl: string;
  readonly diagnosticId: string | null;
  constructor(
    articleUrl: string,
    retryByReloading: boolean,
    diagnosticId: string | null = null,
  ) {
    super(retryByReloading
      ? "The site icon couldn’t be loaded. Reloading the article will also retry loading its icon."
      : "The site icon couldn’t be loaded.");
    this.name = "FaviconLoadError";
    this.articleUrl = articleUrl;
    this.diagnosticId = diagnosticId;
  }
}
