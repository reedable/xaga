import assert from "node:assert/strict";
import { readdir, readFile } from "node:fs/promises";
import { test } from "node:test";

const locationPatterns = [
  /"(?:entries|pdfs)(?:\/[^"\n]*)?"/,
  /\b(?:entries_dir|entry_scan_dirs)\s*\(/,
  /\[[^;]*\bentry_path\s*\([^;]*\bpdf_entry_path\s*\(/s,
];

const repeatsEntryLocations = (source) =>
  locationPatterns.some((pattern) => pattern.test(source));

async function* rustFiles(directory) {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = new URL(
      entry.name + (entry.isDirectory() ? "/" : ""),
      directory
    );
    if (entry.isDirectory()) {
      if (entry.name !== "target") yield* rustFiles(path);
    } else if (entry.name.endsWith(".rs")) {
      yield path;
    }
  }
}

test("Entry location guard detects local paths and duplicate scan roots", () => {
  for (const source of [
    'path.join("entries/example.md")',
    'path.join("pdfs")',
    "let dirs = entry_scan_dirs(path);",
    "let dirs = [entry_path(path, id), pdf_entry_path(path, id)];",
  ]) {
    assert.ok(repeatsEntryLocations(source), source);
  }
  assert.equal(repeatsEntryLocations('path.join("entries-other")'), false);
});

test("only the library location API defines Entry paths and scan roots", async () => {
  const owner = new URL("../crates/xaga-core/src/library.rs", import.meta.url)
    .href;
  for (const tree of ["../crates/", "../src-tauri/"]) {
    for await (const file of rustFiles(new URL(tree, import.meta.url))) {
      if (file.href === owner) continue;
      const source = await readFile(file, "utf8");
      assert.equal(repeatsEntryLocations(source), false, file.pathname);
    }
  }
});
