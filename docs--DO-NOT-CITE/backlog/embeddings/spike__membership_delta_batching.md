# Membership delta-batching threshold

Determine whether rapid individual `AddMember`/`RemoveMember` bursts on the
Interactive tier need batching. Measure a burst before choosing a cutoff.
Wholesale membership edits use `ReconcileMembership`; this investigation concerns
individual deltas rather than the bulk reconciliation path.
