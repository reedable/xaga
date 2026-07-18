import DOMPurify from "dompurify";
import rehypeRaw from "rehype-raw";
import rehypeStringify from "rehype-stringify";
import remarkGfm from "remark-gfm";
import remarkParse from "remark-parse";
import remarkRehype from "remark-rehype";
import { unified } from "unified";

// Security-critical: The input comes from untrusted sources (feed content,
// .md files, etc) and the output is rendered in the app. rehype-raw lets
// embedded HTML through. The DOMPurify pass is what stops script injection;
// it must stay.
//
// Read docs/architecture/security/handling_persisted_content.md before
// making any changes.
export async function markdownToHtml(markdown: string): Promise<string> {
  const file = await unified()
    .use(remarkParse)
    .use(remarkGfm)
    .use(remarkRehype, { allowDangerousHtml: true })
    .use(rehypeRaw)
    .use(rehypeStringify)
    .process(markdown);
  return DOMPurify.sanitize(String(file));
}
