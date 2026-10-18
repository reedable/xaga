import { expect, it } from "vitest";
import { isLocalFaviconReference } from "./isLocalFaviconReference";

it("accepts legacy and shared favicon references", () => {
  expect(isLocalFaviconReference("../images/entry/icon.png")).toBe(true);
  expect(isLocalFaviconReference("../favicons/63528ce68ce9")).toBe(true);
});

it("rejects remote and unrelated references", () => {
  expect(isLocalFaviconReference("https://example.org/favicon.ico")).toBe(false);
  expect(isLocalFaviconReference("../other/favicon.png")).toBe(false);
  expect(isLocalFaviconReference(undefined)).toBe(false);
});
