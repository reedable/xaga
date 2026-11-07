import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";

const root = fileURLToPath(new URL("../", import.meta.url));
const oxlint = path.join(root, "node_modules/.bin/oxlint");

function lint(source) {
  const directory = mkdtempSync(path.join(tmpdir(), "xaga-lint-"));
  try {
    const fixture = path.join(directory, "fixture.tsx");
    writeFileSync(fixture, source);
    const result = spawnSync(
      oxlint,
      [
        "--config",
        path.join(root, ".oxlintrc.json"),
        "--format",
        "json",
        fixture,
      ],
      { cwd: root, encoding: "utf8" }
    );
    assert.ifError(result.error);
    assert.ok(result.status === 0 || result.status === 1, result.stderr);
    return JSON.parse(result.stdout).diagnostics;
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

for (const [name, jsx, violations] of [
  ["direct tooltip", "<Tooltip><IconButton /></Tooltip>", 0],
  [
    "disabled button wrapper",
    "<Tooltip><span><IconButton disabled /></span></Tooltip>",
    0,
  ],
  ["missing tooltip", "<IconButton />", 1],
  ["unrelated wrapper", "<div><IconButton /></div>", 1],
  ["span without tooltip", "<span><IconButton /></span>", 1],
  ["tooltip too far above", "<Tooltip><div><IconButton /></div></Tooltip>", 1],
]) {
  test(`tooltip rule: ${name}`, () => {
    const diagnostics = lint(
      `export default function Example() { return ${jsx}; }`
    );
    assert.equal(diagnostics.length, violations, JSON.stringify(diagnostics));
    for (const diagnostic of diagnostics) {
      assert.match(
        diagnostic.message,
        /IconButton must be wrapped in a Tooltip/
      );
    }
  });
}

test("React Hooks rules still reject conditional hooks", () => {
  const diagnostics = lint(`
    import { useState } from "react";
    export default function Example({ enabled }: { enabled: boolean }) {
      if (enabled) useState(0);
      return null;
    }
  `);
  assert.ok(
    diagnostics.some((diagnostic) => /rules-of-hooks/.test(diagnostic.code)),
    JSON.stringify(diagnostics)
  );
});

test("TypeScript syntax rules still reject explicit any", () => {
  const diagnostics = lint("export type Example = any;");
  assert.ok(
    diagnostics.some((diagnostic) => /no-explicit-any/.test(diagnostic.code)),
    JSON.stringify(diagnostics)
  );
});

test("React Compiler checks still reject mutating props", () => {
  const diagnostics = lint(`
    export default function Example({ value }: { value: { count: number } }) {
      value.count = 1;
      return <div>{value.count}</div>;
    }
  `);
  assert.ok(
    diagnostics.some((diagnostic) => /immutability/.test(diagnostic.code)),
    JSON.stringify(diagnostics)
  );
});

test("module parsing rejects legacy octal literals", () => {
  const diagnostics = lint("export const example = 0123;");
  assert.ok(diagnostics.length > 0);
});
