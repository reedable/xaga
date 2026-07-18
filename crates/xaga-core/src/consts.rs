//! Intentionally empty — this file is a guardrail, not a home for constants.
//!
//! This codebase does NOT collect constants in one place. The convention is the
//! opposite: every constant lives colocated with the concept that owns it, at the
//! narrowest visibility that compiles — tag vocabulary in `tags`, model config in
//! `embeddings::model`, tuning knobs beside the code they tune.
//!
//! If you landed here because you were tempted to "keep the constants together,"
//! that is the tell that something is off. Grouping symbols by what they *are*
//! (constants) instead of by what they *mean* over-widens visibility — a central
//! bucket pressures every value toward `pub` so unrelated modules can reach it,
//! which is exactly backwards. Keep the constant private, next to its meaning.
//!
//! This file should stay empty. Add to it only for a genuinely cross-cutting piece
//! of app identity with no better conceptual home — and even then, prefer a named
//! concept module (e.g. `library`) over a generic "consts" dumping ground.
//!
//! See `docs/principles/least_visibility_principle.md` (Principle 2 — Least
//! Visibility), and `docs/principles/index.md`.
