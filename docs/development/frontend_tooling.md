# Frontend tooling

Use `npm ci` to install the locked toolchain. TypeScript 7 supplies `tsc`;
Oxlint runs lint checks, and Oxfmt handles formatting.

- `npm run lint` checks the existing TypeScript/TSX lint scope.
- `npm run lint:fix` applies available safe lint fixes.
- `npm run format` formats supported files, respecting `.gitignore`.
- `npm run format:check` checks formatting without writing files.
- `npm run typecheck` checks both TypeScript projects.
- `npm run ui:build` checks CSP hashes, checks types, and builds the frontend.
- `npm run test:scripts` includes regression tests for the lint configuration.

## Lint compatibility

`.oxlintrc.json` explicitly carries forward the previous ESLint rule severities
and options rather than relying on Oxlint's different defaults. TypeScript's
compiler handles checks that the previous TypeScript ESLint configuration had
disabled, such as undefined names. Legacy octal literals are rejected by the
parser in modules, so the former `no-octal` rule needs no replacement.

The local JavaScript plugin in `lint-rules/` preserves the requirement that an
`IconButton` be directly inside a `Tooltip`, optionally with a `span` between
them for disabled buttons.

Basic Hooks checks and React Refresh use Oxlint's native rules. React Compiler
checks use `eslint-plugin-react-hooks` through Oxlint's JavaScript plugin API:
several have no native equivalent, and the native implementations of `refs`
and `set-state-in-effect` reported additional errors on code accepted by the
previous plugin. Keeping the original Compiler rules preserves that behavior.
ESLint remains a transitive peer dependency of this plugin; the lint command
does not run ESLint. There is no dependency on `typescript-eslint` or the
TypeScript 5/6 compiler API.

When updating Oxlint, run the lint regression tests and the full lint command.
Reassess native React Compiler rule parity before removing the JavaScript plugin.

## Formatting

`.oxfmtrc.json` preserves the former Prettier settings, including its implicit
80-column print width. Package JSON sorting is disabled to preserve the existing
ordering. Rust manifests and icon bundles keep their existing formatting and
are excluded. Formatting changes should be reviewed separately from behavior changes.
