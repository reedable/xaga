# Compliance: image dependency distribution licenses

Provide dependency notices and MPL source-availability information in app
distributions before shipping shared favicon and image-recovery validation.

## Dependencies and fixture to cover

- `avif-parse` 2.1.0: MPL-2.0.
- `bitreader` 0.3.11, `fallible_collections` 0.5.2, and `leb128` 0.2.7:
  MIT OR Apache-2.0.
- `quick-xml` 0.37.5: MIT.
- `src-tauri/tests/fixtures/color-profile.avif`: third-party fixture offered
  under LGPL 2.1 or BSD-2-Clause. Use the BSD-2-Clause option and verify the
  attribution and license text in the fixture README against the source.

## Work

- Verify licenses and original notices against the exact versions in
  `Cargo.lock`, including transitive dependencies.
- Add required third-party copyright/license notices to the distributed app
  or accompanying distribution materials. Record which license option is used
  for dual-licensed dependencies and preserve the corresponding notices.
- Make the exact MPL-covered source used in the release available to recipients
  and include clear instructions for obtaining it. Retain its MPL notices and
  license information; include any future changes to covered files under MPL.
- Ensure Xaga's proprietary distribution terms preserve recipients' rights to
  the MPL-covered source. MPL permits a proprietary larger work; this does not
  require publishing Xaga's separately authored source files.
- Verify the test fixture's provenance and original attribution. Preserve the
  BSD notices wherever the fixture is redistributed, and confirm whether release
  packaging includes it rather than assuming test-only code guarantees exclusion.
- Verify the packaged release, not just the repository: recipients must receive
  the notices and be able to obtain the matching MPL source using the provided
  instructions. Record the packaging location and verification result.

## Completion criteria

- A built distribution contains accessible notices covering the dependencies
  above and usable source-availability instructions for `avif-parse`.
- The source made available matches the shipped dependency version and includes
  any modifications to MPL-covered files.
- Fixture attribution is preserved in distributions that include the fixture.
- Distribution compliance is verified before shipping the shared image-validator
  change.

References: [MPL 2.0 sections 3.1–3.4](https://www.mozilla.org/en-US/MPL/2.0/),
[Mozilla's distribution FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/),
[MIT license](https://opensource.org/license/mit), and
[BSD-2-Clause license](https://opensource.org/license/bsd-2-clause).

Relevant files: `src-tauri/Cargo.toml`, `Cargo.lock`, `LICENSE`,
`src-tauri/tauri.conf.json`, and `src-tauri/tests/fixtures/README.md`.
