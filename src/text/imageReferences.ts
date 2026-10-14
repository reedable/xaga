import { parseFragment, type DefaultTreeAdapterMap } from "parse5";
import { parse as parseCss, walk as walkCss, generate } from "css-tree";
import { unified } from "unified";
import remarkParse from "remark-parse";
import remarkGfm from "remark-gfm";

export type ImagePatch = {
  start: number;
  end: number;
  original: string;
  url: string;
  css: boolean;
};
export type ImageRecovery = {
  summary: boolean;
  expected: string;
  patches: ImagePatch[];
};
type SyntaxNode = {
  type: string;
  url?: string;
  value?: string;
  children?: SyntaxNode[];
  position?: { start: { offset?: number }; end: { offset?: number } };
};

function remote(value: string, base: string): string | null {
  if (!value.trim()) return null;
  if (value.startsWith("../images/") || value.startsWith("data:")) return null;
  try {
    const url = new URL(value, base);
    return /^https?:$/.test(url.protocol) ? url.href : null;
  } catch {
    return null;
  }
}

// Fetch only image-valued CSS properties, not font or stylesheet resources.
function imageProperty(property: string | undefined): boolean {
  return (
    !!property &&
    /^(?:background(?:-image)?|border-image(?:-source)?|list-style(?:-image)?|(?:-webkit-)?mask(?:-image|-box-image)?|content|cursor|shape-outside)$/i.test(
      property
    )
  );
}

// Map decoded attribute offsets back to the untouched HTML source. CSS locations
// operate on decoded values; entities must not shift the persisted byte spans.
function attributeText(raw: string) {
  let value = "";
  const offsets: number[] = [];
  const entity = /&(?:#[xX][0-9a-fA-F]+|#\d+|[A-Za-z][A-Za-z0-9]+);?/g;
  let cursor = 0;
  for (const match of raw.matchAll(entity)) {
    for (; cursor < match.index!; cursor++) {
      offsets.push(cursor);
      value += raw[cursor];
    }
    const fragment = parseFragment(`<a x="${match[0]}"></a>`);
    const element = fragment.childNodes[0] as DefaultTreeAdapterMap["element"];
    const decoded = element.attrs[0].value;
    for (let j = 0; j < decoded.length; j++) offsets.push(cursor);
    value += decoded;
    cursor += match[0].length;
  }
  for (; cursor < raw.length; cursor++) {
    offsets.push(cursor);
    value += raw[cursor];
  }
  offsets.push(raw.length);
  return { value, offsets };
}

/** Discover source spans without serializing Markdown or HTML back over user text. */
export function imageReferences(source: string, base: string): ImagePatch[] {
  const patches: ImagePatch[] = [];
  const add = (start: number, end: number, value: string, css = false) => {
    const url = remote(value, base);
    if (!url || end <= start) return;
    patches.push({
      start: new TextEncoder().encode(source.slice(0, start)).length,
      end: new TextEncoder().encode(source.slice(0, end)).length,
      original: source.slice(start, end),
      url,
      css,
    });
  };
  const html = (value: string, offset: number) => {
    const fragment = parseFragment(value, { sourceCodeLocationInfo: true });
    const visit = (node: DefaultTreeAdapterMap["node"]) => {
      if ("tagName" in node) {
        if (node.tagName === "style") {
          for (const child of node.childNodes) {
            if (!("value" in child) || !child.sourceCodeLocation) continue;
            try {
              const sheet = parseCss(child.value, { positions: true });
              walkCss(sheet, function (item) {
                if (
                  item.type !== "Url" &&
                  !(
                    item.type === "String" &&
                    this.function?.name.toLowerCase().endsWith("image-set")
                  )
                )
                  return;
                if (
                  !item.loc ||
                  !imageProperty(this.declaration?.property) ||
                  this.atrule?.name.toLowerCase() === "import"
                )
                  return;
                const begin = offset + child.sourceCodeLocation!.startOffset;
                add(
                  begin + item.loc.start.offset,
                  begin + item.loc.end.offset,
                  item.value,
                  true
                );
              });
            } catch {
              /* Keep unrecognized CSS untouched on disk. */
            }
          }
        }
        for (const attr of node.attrs) {
          const location = node.sourceCodeLocation?.attrs?.[attr.name];
          if (!location) continue;
          const attribute = value.slice(
            location.startOffset,
            location.endOffset
          );
          const equals = attribute.indexOf("=");
          if (equals < 0) continue;
          const contentStart =
            equals +
            1 +
            (attribute.slice(equals + 1).match(/^\s*/)?.[0].length ?? 0);
          const quote = attribute[contentStart];
          const quoted = quote === '"' || quote === "'";
          const start = location.startOffset + contentStart + (quoted ? 1 : 0);
          const end = location.endOffset - (quoted ? 1 : 0);
          if (node.tagName === "img" && attr.name === "src")
            add(offset + start, offset + end, attr.value);
          if (attr.name === "style") {
            const decoded = attributeText(value.slice(start, end));
            try {
              const ast = parseCss(decoded.value, {
                context: "declarationList",
                positions: true,
              });
              walkCss(ast, function (item) {
                if (
                  item.type !== "Url" &&
                  !(
                    item.type === "String" &&
                    this.function?.name.toLowerCase().endsWith("image-set")
                  )
                )
                  return;
                if (!item.loc || !imageProperty(this.declaration?.property))
                  return;
                add(
                  offset + start + decoded.offsets[item.loc.start.offset],
                  offset + start + decoded.offsets[item.loc.end.offset],
                  item.value,
                  true
                );
              });
            } catch {
              /* Unknown CSS is left untouched on disk. */
            }
          }
        }
      }
      if ("childNodes" in node) node.childNodes.forEach(visit);
    };
    fragment.childNodes.forEach(visit);
  };
  const ast = unified()
    .use(remarkParse)
    .use(remarkGfm)
    .parse(source) as SyntaxNode;
  const visit = (node: SyntaxNode) => {
    const start = node.position?.start.offset;
    const end = node.position?.end.offset;
    if (start !== undefined && end !== undefined) {
      if (node.type === "html") html(source.slice(start, end), start);
      if (node.type === "image" && node.url) {
        const raw = source.slice(start, end);
        // The parser has already established a valid inline image. Find its
        // destination, respecting escaped brackets in the alternative text.
        const close = raw.match(/(?<!\\)\]\(\s*/);
        if (close?.index !== undefined) {
          let begin = close.index + close[0].length;
          const angle = raw[begin] === "<";
          if (angle) begin++;
          let finish = begin;
          let depth = 0;
          for (; finish < raw.length; finish++) {
            const char = raw[finish];
            if (char === "\\") {
              finish++;
              continue;
            }
            if (angle ? char === ">" : depth === 0 && /[\s)]/.test(char)) break;
            if (!angle && char === "(") depth++;
            if (!angle && char === ")") depth--;
          }
          add(start + begin, start + finish, node.url);
        }
      }
    }
    node.children?.forEach(visit);
  };
  visit(ast);
  return patches.sort((a, b) => a.start - b.start);
}

/** Resolve CSS resources for display; unknown/remote resource-bearing values
 * are removed rather than delegated to the reader's network stack. */
export function localCss(
  style: string,
  resolve: (url: string) => string | null,
  stylesheet = false
): string {
  try {
    const ast = parseCss(style, {
      context: stylesheet ? "stylesheet" : "declarationList",
    });
    walkCss(ast, {
      visit: "Atrule",
      enter(node, item, list) {
        if (node.name.toLowerCase() === "import") list.remove(item);
      },
    });
    walkCss(ast, {
      visit: "Declaration",
      enter(declaration, item, list) {
        let unavailable = false;
        walkCss(declaration, function (node) {
          if (node.type === "Raw") unavailable = true;
          if (
            node.type === "Url" ||
            (node.type === "String" &&
              this.function?.name.toLowerCase().endsWith("image-set"))
          ) {
            const local = resolve(node.value);
            if (local) node.value = local;
            else unavailable = true;
          }
        });
        if (unavailable) list.remove(item);
      },
    });
    return generate(ast);
  } catch {
    return "";
  }
}
