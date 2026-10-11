import { describe, expect, it, vi } from "vitest";
import { inlineEntryImages } from "./inlineEntryImages";

const DATA_URI =
  "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+M8AAAMBAQDJ/pLvAAAAAElFTkSuQmCC";

describe("inlineEntryImages", () => {
  // The backwards-compatibility guarantee: an entry with no ../images reference
  // is returned byte-identical and never triggers a resolve call.
  it("returns input identical and never calls resolve when there are no local refs", async () => {
    const resolve = vi.fn();
    const html =
      '<p>text</p><img src="https://cdn.test/a.jpg"><img src="data:image/gif;base64,AAAA">';
    const out = await inlineEntryImages(html, "e1", resolve);
    expect(out).toBe(html);
    expect(resolve).not.toHaveBeenCalled();
  });

  it("replaces a local ref with the resolved data URI", async () => {
    const resolve = vi.fn().mockResolvedValue([DATA_URI]);
    const html = '<p><img src="../images/e1/4f2a91c3e0b7.png"></p>';
    const out = await inlineEntryImages(html, "e1", resolve);
    expect(resolve).toHaveBeenCalledWith("e1", [
      "../images/e1/4f2a91c3e0b7.png",
    ]);
    expect(out).toContain(`src="${DATA_URI}"`);
    expect(out).not.toContain("../images/");
  });

  it("leaves an https src untouched even when a sibling local ref is rewritten", async () => {
    const resolve = vi.fn().mockResolvedValue([DATA_URI]);
    const html =
      '<img src="https://cdn.test/keep.jpg"><img src="../images/e1/abc123abc123.png">';
    const out = await inlineEntryImages(html, "e1", resolve);
    expect(out).toContain('src="https://cdn.test/keep.jpg"');
    expect(resolve).toHaveBeenCalledWith("e1", [
      "../images/e1/abc123abc123.png",
    ]);
    // Only the local ref was offered for resolution.
    expect(resolve.mock.calls[0][1]).toHaveLength(1);
  });

  it("leaves the original ref in place when resolve returns null", async () => {
    const resolve = vi.fn().mockResolvedValue([null]);
    const html = '<img src="../images/e1/missing000000.png">';
    const out = await inlineEntryImages(html, "e1", resolve);
    expect(out).toContain('src="../images/e1/missing000000.png"');
  });

  it("returns the input unchanged when resolve throws", async () => {
    const resolve = vi.fn().mockRejectedValue(new Error("ipc down"));
    const html = '<p><img src="../images/e1/abc123abc123.png"></p>';
    const out = await inlineEntryImages(html, "e1", resolve);
    expect(out).toContain('src="../images/e1/abc123abc123.png"');
  });

  it("dedupes repeated refs into a single resolve entry", async () => {
    const resolve = vi.fn().mockResolvedValue([DATA_URI]);
    const ref = "../images/e1/dupdupdupdup.png";
    const html = `<img src="${ref}"><img src="${ref}">`;
    const out = await inlineEntryImages(html, "e1", resolve);
    expect(resolve.mock.calls[0][1]).toEqual([ref]);
    // Both occurrences are rewritten.
    expect(out.split(DATA_URI).length - 1).toBe(2);
  });

  it("does not send unrecognized relative srcs to resolve", async () => {
    const resolve = vi.fn().mockResolvedValue([]);
    const html =
      '<img src="/root.png"><img src="//host/x.png"><img src="./sibling.png"><img src="images/x.png">';
    const out = await inlineEntryImages(html, "e1", resolve);
    expect(out).toBe(html);
    expect(resolve).not.toHaveBeenCalled();
  });
});


it("reports missing local images once without recovering or fetching remote URLs", async () => {
  const resolve = vi.fn().mockResolvedValue([null, DATA_URI, null]);
  const missing = vi.fn();
  const html = '<img alt="one" src="../images/e1/111111111111"><img src="../images/e1/222222222222"><img src="../images/e1/333333333333">';
  const out = await inlineEntryImages(html, "e1", resolve, missing);
  expect(missing).toHaveBeenCalledOnce();
  expect(resolve).toHaveBeenCalledOnce();
  expect(out).toContain('alt="one"');
  expect(out).toContain(DATA_URI);
  expect(out).toContain('../images/e1/111111111111');
  expect(out).not.toContain('https:');
});

it("reports resolver failure and leaves stored image paths untouched", async () => {
  const resolve = vi.fn().mockRejectedValue(new Error("read failed"));
  const missing = vi.fn();
  const html = '<img src="../images/e1/111111111111">';
  expect(await inlineEntryImages(html, "e1", resolve, missing)).toBe(html);
  expect(missing).toHaveBeenCalledOnce();
});
