# Distribute image dependency licenses

## Summary

Release artifacts do not provide notices or source-availability instructions for the image-validation dependency chain. The shipped graph includes MPL-2.0 `avif-parse` 2.1.0, its MIT OR Apache-2.0 dependencies `bitreader` 0.3.11, `fallible_collections` 0.5.2, and `leb128` 0.2.7, and MIT `quick-xml` 0.37.5. The AVIF test fixture is available under BSD-2-Clause but should not be assumed absent from an artifact without inspecting the package.

## Requirements

- Generate notices from the exact dependency graph and license texts used by each release.
- Tell recipients how to obtain the matching MPL-covered source and preserve MPL notices and rights.
- Include modifications to MPL-covered files in the offered source.
- Preserve the selected MIT, Apache-2.0, and BSD-2-Clause notices where their terms require them.
- Include the fixture's BSD-2-Clause attribution if any distributed artifact contains it.
- Verify compliance against the packaged artifact, not only repository files.

## Remaining work

- Choose and configure a reproducible notice-generation and packaging mechanism for Rust and frontend dependencies.
- Define where release recipients access notices and the matching `avif-parse` source.
- Inspect each packaged target for included dependencies, notices, source instructions, and test fixtures.
- Add a release check that fails when the dependency graph changes without regenerated compliance material.
- Obtain legal review before external distribution if the intended notice or source-offer mechanism remains uncertain.

## References

- `src-tauri/Cargo.toml`
- `Cargo.lock`
- `src-tauri/tauri.conf.json`
- `src-tauri/tests/fixtures/README.md`
- [Mozilla Public License 2.0](https://www.mozilla.org/en-US/MPL/2.0/)
- [Mozilla MPL FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/)
- [AVIF fixture source](https://github.com/link-u/avif-sample-images/blob/master/README.md)
