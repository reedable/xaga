# Show article reload progress and failures

## Summary

Article reload has no visible in-progress state, and a later rendering warning can replace the refresh failure and its diagnostic link even though the saved article remains displayed.

## Requirements

- Indicate reload progress without hiding the saved article.
- Keep Refresh always visible beside Bookmark, independent of address-bar hover.
- During reload, rotate the Refresh icon and change SR-only feedback and title
  text to "Reload in progress...".
- Follow the motion convention: keep the icon static in base styles and enable
  rotation only under `prefers-reduced-motion: no-preference`. The title change
  supplies nonanimated sighted feedback; preserve SR-only feedback independently.
- Keep Refresh enabled during reload; ignore additional activations without
  starting or restarting work and without additional feedback.
- Preserve the saved article when retrieval fails.
- Keep the refresh failure and its diagnostics accessible when rendering reports a separate image problem.
- Queue notifications in the shared error sink, showing one toast at a time in
  arrival order. Later notifications must not overwrite the current notification
  or its diagnostic link. Apply this behavior to all accepted error-sink
  notifications, not only reload failures.
- Retain each queued notification's message, severity, and diagnostic identity
  independently; its Details action must open its own diagnostic record.
- Diagnostic toasts persist until dismissed; other toasts expire after six
  seconds of display. Advance the queue when the current toast closes.
- Opening Details does not dismiss the current toast or advance the queue.
  Keep that toast current while its Details dialog is open; closing the dialog
  alone does not dismiss the toast.
- Do not automatically retry network requests.
- Clear or replace feedback only according to an explicit lifecycle.

## Remaining work

- Implement queued toasts in the shared error sink with the specified timing,
  dismissal, diagnostic identity, and Details-dialog behavior.
- Define feedback transitions for success, failure, dismissal, explicit retry,
  navigation, and cancellation.
- Review both reload paths (`ItemView` URL resolution and `ArticleReader`'s
  imperative reload) and define consistent behavior, including saved PDFs and
  rejected downloads when saved content is already displayed.
- Implement progress, retained-content behavior, and diagnostic ownership for
  the agreed lifecycle without allowing an obsolete attempt to update another
  article's feedback.
- Test successful reload, failed retrieval with retained content, failure
  followed by an image or favicon warning, explicit retry, and navigation or
  cancellation during reload. Verify Refresh visibility, screen-reader progress
  feedback, and ignored repeated activations.
- Test generic notification ordering, independent diagnostic links, dismissal,
  timeout, and notifications arriving while Details is open.

## References

- `src/components/content/ItemView.tsx`
- `src/hooks/useContentLoader.ts`
- `src/components/content/ArticleReader.tsx`
- `src/components/content/Toolbar.tsx`
- `src/providers/ErrorSinkProvider.tsx`
- `docs/architecture/article_loading_diagnostics.md`
- `docs/conventions/motion.md`
