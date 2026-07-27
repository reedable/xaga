# Fresh-reindex memory ceiling: content-carrying jobs on an unbounded queue (pending — deferred with fix direction)

## Current state

Recovery emits domain objects onto the unbounded embed queue: a fresh reindex enqueues the whole corpus as content-carrying embed jobs. The queue's memory footprint scales with corpus size — estimated at roughly 10–20 GB at one million entries, which is inside the design target (`docs/architecture/design_scale.md`). This was explicitly deferred (not a hidden judgment) during the decoupling work.

## Recorded fix direction

Enqueue the id and re-read the file at embed time, instead of carrying the content on the job. A batch-carrying job variant would not fix it — the content is the weight, not the job count.

## Interactions

`job_payload_modeling.md` (what a Job carries is the same decision surface); `embed_backfill_per_row_costs.md` (the same backfill path's commit costs).
