# Membership delta-batching threshold (pending — possibly superseded)

As deferred at taxonomy-ratification time: `AddMember`/`RemoveMember` ride the Interactive tier per-delta; a batch cutoff would be added only if a burst (e.g. a rapid bulk curation) proves it necessary.

Status caveat, honestly marked: the bulk paths were later addressed separately — wholesale edits go through `ReconcileMembership`, and the reconcile fan-out is chunked/checkpointed (`docs/architecture/crash_consistency.md`) — so the remaining question is only whether rapid _individual_ add/remove bursts on the Interactive tier ever need batching. No burst has been observed; verify the concern is still live before designing anything.
