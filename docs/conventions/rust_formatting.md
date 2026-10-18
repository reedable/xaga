# Rust formatting

Rust source follows the repository toolchain's standard `rustfmt` output across
the entire Cargo workspace. Do not preserve or introduce hand-formatted
exceptions when editing a file.

Run `cargo fmt --all` after changing Rust source. Before completing the work,
run `cargo fmt --all --check`; a formatting failure anywhere in the workspace is
a repository failure rather than unrelated local drift.

Formatting-only changes must remain mechanical. Review the formatter diff for
unexpected non-formatting edits, and run checks proportionate to the affected
workspace crates.
