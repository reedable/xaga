/// <reference lib="webworker" />

import { Readability } from '@mozilla/readability'
import { unified } from 'unified'
import rehypeParse from 'rehype-parse'
import rehypeRemark from 'rehype-remark'
import remarkGfm from 'remark-gfm'
import remarkStringify from 'remark-stringify'

export type EntryRequest = {
  url: string
  html: string
}

export type EntryResult =
  | { title: string; markdown: string }
  | { error: string }

self.onmessage = async (event: MessageEvent<EntryRequest>) => {
  const { url, html } = event.data

  try {
    // Parse HTML into a DOM tree (DOMParser is available in Workers on Safari 15.4+)
    const parser = new DOMParser()
    const doc = parser.parseFromString(html, 'text/html')

    // Set base URL so Readability resolves relative links correctly
    const base = doc.createElement('base')
    base.href = url
    doc.head.prepend(base)

    const entry = new Readability(doc).parse()
    if (!entry) {
      self.postMessage({ error: 'Readability could not extract an entry' } satisfies EntryResult)
      return
    }

    // Convert entry HTML → Markdown
    const file = await unified()
      .use(rehypeParse, { fragment: true })
      .use(rehypeRemark)
      .use(remarkGfm)
      .use(remarkStringify)
      .process(entry.content ?? '')

    self.postMessage({
      title: entry.title ?? '',
      markdown: String(file),
    } satisfies EntryResult)
  } catch (err) {
    self.postMessage({ error: String(err) } satisfies EntryResult)
  }
}
