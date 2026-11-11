# Formatting

Formatting is a completion requirement for every story, including documentation,
investigations, and formatting-only work. These shared checks apply even when a
story does not repeat them in its requirements.

Run `npm run format` after editing files supported by the repository formatter.
Review the resulting diff to confirm formatting changes are mechanical and
preserve behavior and document meaning.

After the final edits and before reporting completion or deleting a resolved
transient task, run `npm run format:check` across the repository. A formatting
failure anywhere in the checked files is a repository failure; resolve it before
completing the story. Rerun the check if subsequent edits change checked files.

For Rust changes, also follow [Rust formatting](rust_formatting.md), including
`cargo fmt --all` and `cargo fmt --all --check`. The npm formatter does not
replace these Cargo workspace checks.
