import { prepareEntryImages } from "../text/prepareEntryImages";
import type { ResolveEntryImages } from "../text/inlineEntryImages";
import { markdownToHtml } from "../text/markdownPipeline";
import type { Article, ListItem } from "../types";

/** Resolve each segment using the reader's non-overlapping, per-text-node occurrence convention. */
export async function articleScopeItems(
  article: Article,
  resolveImages: ResolveEntryImages
): Promise<ListItem[]> {
  // Template content stays inert: ordering must never load article resources.
  const template = document.createElement("template");
  template.innerHTML = await prepareEntryImages(
    await markdownToHtml(article.content || article.description || ""),
    article.id,
    resolveImages
  );
  const nodes: { text: string; offset: number }[] = [];
  const walker = document.createTreeWalker(
    template.content,
    NodeFilter.SHOW_TEXT,
    {
      acceptNode(node) {
        for (
          let parent = node.parentElement;
          parent;
          parent = parent.parentElement
        ) {
          if (
            ["SCRIPT", "STYLE"].includes(parent.tagName) ||
            parent.getAttribute("aria-hidden") === "true"
          )
            return NodeFilter.FILTER_REJECT;
        }
        return NodeFilter.FILTER_ACCEPT;
      },
    }
  );
  let offset = 0;
  while (walker.nextNode()) {
    const text = walker.currentNode.nodeValue ?? "";
    nodes.push({ text, offset });
    offset += text.length;
  }
  const position = (text: string, occurrence: number | undefined) => {
    if (
      !text ||
      occurrence === undefined ||
      !Number.isInteger(occurrence) ||
      occurrence < 0
    )
      return Infinity;
    let count = 0;
    for (const node of nodes) {
      let start = 0;
      let index: number;
      while ((index = node.text.indexOf(text, start)) !== -1) {
        if (count++ === occurrence) return node.offset + index;
        start = index + text.length;
      }
    }
    return Infinity;
  };
  const annotations = article.annotations.map((annotation) => ({
    annotation,
    position: Math.min(
      ...annotation.text.map((text, index) =>
        position(text, annotation.occurrence[index])
      )
    ),
  }));
  annotations.sort((a, b) => a.position - b.position);
  return [
    article,
    ...annotations.map(({ annotation }) => ({
      id: annotation.id,
      reference: article.id,
      title: annotation.title,
      color: annotation.color,
      createdAt: annotation.createdAt,
      url: article.url,
    })),
  ];
}
