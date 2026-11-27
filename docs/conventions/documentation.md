# Documentation

## Organization

Durable documentation records the system’s rules, design, terminology, and behavior. It remains useful after a task is complete and may be cited by code and other durable documents.

Recurring maintenance procedures live permanently under
[`docs/maintenance/`](../maintenance/index.md). Follow its
[review trigger](../maintenance/index.md#review-trigger) when entering `x.y.0-dev`:
remind the user to review and select maintenance work for the new cycle.
They define repeatable work and are retained after each run. Each procedure states
its purpose, scope, cadence or trigger (explicitly undecided until agreed), steps,
and completion criteria. Keep procedures current without accumulating run history;
report each run's results in the work report. Create transient tasks for findings
requiring separate decisions or follow-up work. Maintenance runs follow the
applicable verification and completion gate.

Each durable documentation directory has an index. Keep indexes terse and focused
on navigation. Place detailed guidance in the linked documents.

## Stories and ideas

See the [Xaga Stories index](https://github.com/reedable/xaga-stories#xaga-stories)
for story organization, lifecycle, structure, filenames, readiness, and
verification handoff.

## Writing style

Follow the [writing style guide](writing_style.md) for shared language,
terminology, and UI copy. Technical documentation follows its technical-audience
guidance; the User Guide follows the same reader-facing guidance as product
training.
