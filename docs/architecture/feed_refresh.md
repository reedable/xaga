# Feed refresh

One `refresh_all_feeds` request returns `{ feeds, failures }`. `feeds` contains
successful refreshes, including feeds with no new articles. Each failure carries
its saved feed ID and title, a stable error code, and the domain error's safe
message. Detailed network/parser diagnostics remain in native logs.

Successful feeds still emit per-feed updates and enqueue newly created articles
for indexing. The aggregate refresh event remains unchanged. A failed feed does
not discard another feed's success. The frontend retains its current ListView
membership until the existing explicit navigation/filter/refresh rules apply.

The feed hook sends one consolidated notification to the shared error sink per
failed pass, naming affected feeds and reasons and directing the user to Refresh
to retry. Whole-request failures use the same surface. Cancellation produces no
notification; loading state clears in every outcome. Both toolbar spinners follow
the request lifetime rather than a timer. No all-success message is inferred from
an empty success list.

When Refresh on startup is enabled, the main layout performs one refresh after
initial settings and writable library mode are known. Read-only startup skips
this automatic write. Manual refresh uses the same hook and error contract;
backend refusal remains authoritative if a residual request reaches it.

Add Feed can retain a saved subscription when the first refresh fails. The
command returns the saved Feed ID with the failure, and a retry refreshes that
ID using its saved address. The form retains the ID across later name or
shortcut failures so it can resume the unfinished step. Adding or renaming a
Feed returns the committed Feed even when reading shortcut state for its
projection fails.
