# Maintenance

Permanent procedures for recurring engineering work, governed by the
[documentation convention](../conventions/documentation.md).

## Review trigger

When entering a new minor development cycle at `x.y.0-dev`, remind the user to
review the maintenance procedures below and decide which sweeps to run. This is
a reminder to select work, not automatic authorization to execute every sweep or
refactor its findings. Schedule selected work during development, before release
candidate stabilization. The trigger follows version evolution; there is no
calendar or patch-count fallback.

## Procedures

1. [Dead-code sweep](dead_code_sweep.md) — unused code and unnecessarily broad visibility.
2. [Hotspot sweep](hotspot_sweep.md) — frequently changed, large modules and
   evidence of responsibility growth that may warrant refactoring.
