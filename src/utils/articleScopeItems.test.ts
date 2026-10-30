import { expect, it } from "vitest";
import type { Annotation, Article } from "../types";
import { articleScopeItems as loadScopeItems } from "./articleScopeItems";
const articleScopeItems = (article: Article) =>
  loadScopeItems(article, async () => []);

function annotation(
  id: string,
  text: string[],
  occurrence = text.map(() => 0)
): Annotation {
  return {
    __kind: "annotation",
    id,
    title: text.join(" "),
    text,
    occurrence,
    reference: "article",
    createdAt: "2026-10-29",
    color: "yellow",
    content: "",
  };
}
function article(
  annotations: Annotation[],
  content = "First **bold** passage.\n\nRepeated. Repeated.\n\nLast passage."
): Article {
  return {
    __kind: "entry",
    id: "article",
    title: "Title",
    url: "https://example.org",
    createdAt: "2026-10-29",
    isBookmarked: false,
    isPinned: false,
    content,
    annotations,
  };
}

it("places the article first, resolves rendered text and repeated passages, and puts unresolved anchors last", async () => {
  const result = await articleScopeItems(
    article([
      annotation("missing", ["absent"]),
      annotation("second-repeat", ["Repeated"], [1]),
      annotation("last", ["Last"]),
      annotation("bold", ["bold"]),
      annotation("first-repeat", ["Repeated"], [0]),
    ])
  );
  expect(result.map((item) => item.id)).toEqual([
    "article",
    "bold",
    "first-repeat",
    "second-repeat",
    "last",
    "missing",
  ]);
});

it("orders multi-segment annotations by their earliest resolved segment even when another segment is missing", async () => {
  const result = await articleScopeItems(
    article([
      annotation("later", ["Last"]),
      annotation("partial", ["absent", "First", "Last"]),
      annotation("none", ["missing", "absent"]),
    ])
  );
  expect(result.map((item) => item.id)).toEqual([
    "article",
    "partial",
    "later",
    "none",
  ]);
});

it("matches occurrences within text nodes, omits hidden text, and rejects invalid anchors without hanging", async () => {
  const result = await articleScopeItems(
    article(
      [
        annotation("cross-node", ["one two"]),
        annotation("empty", [""]),
        annotation("negative", ["three"], [-1]),
        annotation("hidden", ["secret"]),
        annotation("valid", ["three"]),
      ],
      'one **two** three <span aria-hidden="true">secret</span>'
    )
  );
  expect(result[1].id).toBe("valid");
  expect(result.slice(2)).toHaveLength(4);
});

it("counts the same unavailable-image fallback text as the reader", async () => {
  const result = await articleScopeItems(
    article(
      [
        annotation("paragraph", ["Repeated"], [1]),
        annotation("image", ["Repeated"], [0]),
      ],
      "![Repeated](https://example.org/unavailable.png)\n\nRepeated."
    )
  );
  expect(result.map((item) => item.id)).toEqual([
    "article",
    "image",
    "paragraph",
  ]);
});

it("uses the full body even when a summary-only anchor resolves, and falls back to summary only without a body", async () => {
  const value = article([
    annotation("summary", ["Summary passage"]),
    annotation("body", ["Last"]),
  ]);
  value.description = "Summary passage.";
  expect((await articleScopeItems(value)).map((item) => item.id)).toEqual([
    "article",
    "body",
    "summary",
  ]);
  value.content = "";
  expect((await articleScopeItems(value)).map((item) => item.id)).toEqual([
    "article",
    "summary",
    "body",
  ]);
});
