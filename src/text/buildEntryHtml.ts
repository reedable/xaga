import readerCss from "../../../lliwio-css/src/index.css?inline";
import { type ThemeMode } from "../providers/AppearanceContext";
import { FONT_FAMILY_CSS, type FontFamily } from "../utils/FontFamily";

// Annotation highlight colours for the iframe document.
// These are intentionally kept here rather than imported from colors.module.css
// because the iframe is a separate document and cannot use CSS Modules.
const MARK_COLORS_LIGHT: Record<string, { bg: string; accent: string }> = {
  red: { bg: "rgba(255,80,80,0.35)", accent: "rgba(210,40,40,0.60)" },
  orange: { bg: "rgba(255,150,0,0.40)", accent: "rgba(210,120,0,0.65)" },
  yellow: { bg: "rgba(255,214,0,0.40)", accent: "rgba(255,180,0,0.60)" },
  green: { bg: "rgba(60,190,60,0.35)", accent: "rgba(30,150,30,0.65)" },
  blue: { bg: "rgba(60,130,255,0.35)", accent: "rgba(30,90,220,0.65)" },
  violet: { bg: "rgba(160,70,255,0.35)", accent: "rgba(120,30,210,0.50)" },
};
const MARK_COLORS_DARK: Record<string, { bg: string; accent: string }> = {
  red: { bg: "rgba(255,80,80,0.80)", accent: "rgba(255,130,130,0.75)" },
  orange: { bg: "rgba(255,160,40,0.65)", accent: "rgba(255,190,80,0.75)" },
  yellow: { bg: "rgba(255,220,40,0.55)", accent: "rgba(255,230,80,0.75)" },
  green: { bg: "rgba(60,190,60,0.70)", accent: "rgba(110,230,110,0.75)" },
  blue: { bg: "rgba(60,160,255,0.75)", accent: "rgba(120,180,255,0.75)" },
  violet: { bg: "rgba(200,100,255,0.80)", accent: "rgba(210,140,255,0.75)" },
};

// ── postMessage type constants ────────────────────────────────────────────────

/** Existing: iframe notifies parent that a link was clicked. */
export const OPEN_LINK_URL = "open-url";

/** Iframe notifies parent that the user has selected text. */
export const SELECTION_MADE = "selection-made";

/** Iframe notifies parent that the selection was cleared. */
export const SELECTION_CLEARED = "selection-cleared";

/** Iframe notifies parent that the user clicked an existing highlight mark. */
export const HIGHLIGHT_CLICKED = "highlight-clicked";

/** Iframe notifies parent that Cmd-F was pressed inside the iframe. */
export const FIND_OPEN = "find-open";

/** Iframe notifies parent of the current find result count and position. */
export const FIND_RESULT = "find-result";

/** Parent → iframe: execute a text search. */
export const FIND_IN_PAGE = "find-in-page";

/** Parent → iframe: move to next (+1) or previous (−1) match. */
export const FIND_NAVIGATE = "find-navigate";

/** Parent → iframe: clear all find highlights. */
export const FIND_CLEAR = "find-clear";

/** Parent → iframe: scroll the first mark with the given annotationId into view. */
export const SCROLL_TO_ANNOTATION = "scroll-to-annotation";

// ── Types ─────────────────────────────────────────────────────────────────────

export interface HtmlReaderStyle {
  themeMode: ThemeMode;
  fontFamily: FontFamily;
  fontSize: number;
  lineHeight: number;
}

/** Minimal annotation shape passed into the iframe HTML template. */
export interface SerializedAnnotation {
  id: string;
  text: string[];
  occurrence: number[];
  color: string;
  note: string | null;
}

// ── Builder ───────────────────────────────────────────────────────────────────

export function buildEntryHtml(
  bodyHtml: string,
  style: HtmlReaderStyle,
  annotations: SerializedAnnotation[] = []
): string {
  const { themeMode, fontFamily, fontSize, lineHeight } = style;
  const prefersDark = themeMode === "dark";
  const fg = "var(--lliw-semanticColor__color--primary)";
  const fontStack = FONT_FAMILY_CSS[fontFamily];

  // Embed annotations in a non-executed <script type="application/json"> data
  // island (see the return below). See
  // docs/architecture/security/handling_persisted_content.md, "Rendering," for
  // why this must stay out of the executable <script> and why "<" is escaped.
  //
  // If you change anything inside the executable <script> below, the bundled CSP
  // hash MUST be regenerated: run `node scripts/csp-hash.mjs` and paste the
  // printed value into app.security.csp (script-src) in src-tauri/tauri.conf.json.
  const annotationsJson = JSON.stringify(annotations).replace(/</g, "\\u003c");

  return `<!DOCTYPE html><html><head><meta charset="utf-8"><style>
    ${readerCss}
    :root {
      color-scheme: ${themeMode};
      --lliw-typography__font-family: ${fontStack};
      --lliw-typography__font-size: ${fontSize}px;
      --lliw-typography__rhythm-unit: ${fontSize * lineHeight}px;
    }
    body{margin:0 0 4.5rem 0;padding:0.5rem 2rem 2rem}
    article{max-width:680px;margin:0 auto}
    img{max-width:100%;height:auto}
    .xaga-unavailable-image{display:block;padding:.5em 0;white-space:pre-wrap}
    .xaga-image-title{display:block;font-size:.9em;margin-top:.25em}
    audio,video{display:none}
    a{cursor:pointer}
    table{border-collapse:collapse;width:100%}
    th,td{border:1px solid var(--lliw-semanticColor__color--tertiary);padding:0.4rem 0.6rem}
    ${Object.entries(prefersDark ? MARK_COLORS_DARK : MARK_COLORS_LIGHT)
      .map(
        ([name, t]) =>
          `mark[data-ann-color="${name}"]{background:${t.bg};border-radius:2px;cursor:pointer;outline:1px solid ${t.accent}${prefersDark ? ";color:#000000" : ""}}` +
          `mark[data-ann-color="${name}"]:hover{background:${t.accent};color:${prefersDark ? "#000000" : fg}}`
      )
      .join("")}
    mark.xf{background:${prefersDark ? "#7b6200" : "#fff176"};border-radius:2px;color:inherit}
    mark.xf.xfa{background:${prefersDark ? "#e65100" : "#ff9800"};border-radius:2px;color:${prefersDark ? "#fff" : "#000"};outline:2px solid ${prefersDark ? "#ff6d00" : "#e65100"}}
  </style></head><body><article>${bodyHtml}</article><script type="application/json" id="__xaga_annotations__">${annotationsJson}</script><script>
    document.addEventListener('error', function(event) {
      const img = event.target;
      if (!(img instanceof HTMLImageElement)) return;
      const alternative = img.getAttribute('alt')?.trim();
      const title = img.getAttribute('title')?.trim();
      const fallback = document.createElement('span');
      fallback.className = 'xaga-unavailable-image';
      fallback.textContent = alternative || title || 'Image unavailable';
      if (alternative && title && title !== alternative) {
        const detail = document.createElement('span');
        detail.className = 'xaga-image-title';
        detail.textContent = title;
        fallback.append(detail);
      }
      img.replaceWith(fallback);
    }, true);
    var __ANNOTATIONS__ = JSON.parse(document.getElementById('__xaga_annotations__').textContent || '[]');

    // ── Helpers ────────────────────────────────────────────────────────────────

    // Build a TreeWalker that visits only text nodes outside <script>, <style>,
    // and existing <mark data-ann-id> elements (to avoid double-counting).
    function makeTextWalker(root) {
      return document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
        acceptNode: function(node) {
          var p = node.parentElement;
          while (p && p !== root) {
            var tag = p.tagName ? p.tagName.toUpperCase() : '';
            if (tag === 'SCRIPT' || tag === 'STYLE') return NodeFilter.FILTER_REJECT;
            if (tag === 'MARK' && p.dataset && p.dataset.annId) return NodeFilter.FILTER_REJECT;
            p = p.parentElement;
          }
          return NodeFilter.FILTER_ACCEPT;
        }
      });
    }

    // Block-level tags that delimit segment boundaries.
    var BLOCK_TAGS = {P:1,H1:1,H2:1,H3:1,H4:1,H5:1,H6:1,LI:1,BLOCKQUOTE:1,TD:1,TH:1,PRE:1,DIV:1,SECTION:1,ARTICLE:1};
    function closestBlock(node) {
      var el = node.parentElement;
      while (el && el !== document.body) {
        if (BLOCK_TAGS[el.tagName ? el.tagName.toUpperCase() : '']) return el;
        el = el.parentElement;
      }
      return document.body;
    }

    // Apply one segment of an annotation highlight to the document.
    // segText is the exact string to find; targetOcc is the 0-based occurrence index.
    function applyHighlightSegment(id, segText, targetOcc, color, note) {
      var walker = makeTextWalker(document.body);
      var count = 0;
      var node;

      while ((node = walker.nextNode())) {
        var text = node.nodeValue;
        var start = 0;
        var idx;
        while ((idx = text.indexOf(segText, start)) !== -1) {
          if (count === targetOcc) {
            try {
              var range = document.createRange();
              range.setStart(node, idx);
              range.setEnd(node, idx + segText.length);
              var mark = document.createElement('mark');
              mark.dataset.annId = id;
              mark.dataset.annText = segText;
              mark.dataset.annColor = color;
              if (note) mark.title = note;
              range.surroundContents(mark);

              (function(m) {
                m.addEventListener('click', function(e) {
                  if (e.target && e.target.closest && e.target.closest('a[href]')) return;
                  e.stopPropagation();
                  var rect = m.getBoundingClientRect();
                  window.parent.postMessage({
                    type: '${HIGHLIGHT_CLICKED}',
                    annotationId: m.dataset.annId,
                    text: m.dataset.annText,
                    rect: { top: rect.top, left: rect.left, width: rect.width, height: rect.height }
                  }, '*');
                });
              })(mark);
            } catch (e) {
              console.warn('[xaga] Could not apply highlight segment:', segText, e);
            }
            return;
          }
          count++;
          start = idx + segText.length;
        }
      }
    }

    // Apply all segments of an annotation (ann.text[] / ann.occurrence[]).
    function applyHighlight(ann) {
      for (var i = 0; i < ann.text.length; i++) {
        applyHighlightSegment(ann.id, ann.text[i], ann.occurrence[i], ann.color, ann.note);
      }
    }

    // Remove all highlight marks with the given annotationId, unwrapping text content.
    function removeHighlight(annotationId) {
      var marks = document.querySelectorAll('mark[data-ann-id="' + annotationId + '"]');
      for (var i = 0; i < marks.length; i++) {
        var mark = marks[i];
        var parent = mark.parentNode;
        while (mark.firstChild) parent.insertBefore(mark.firstChild, mark);
        parent.removeChild(mark);
      }
      document.body.normalize();
    }

    // Collect text segments and their occurrence indices from the current selection.
    // Splits at block-level boundaries so each segment can be highlighted independently.
    // Returns { texts: string[], occurrences: number[] } or null when no selection.
    function getSegmentsAndOccurrences() {
      var sel = window.getSelection();
      if (!sel || sel.isCollapsed) return null;
      var range = sel.getRangeAt(0);

      // Collect all text nodes in document order (for occurrence counting).
      var allNodes = [];
      var walkerAll = makeTextWalker(document.body);
      var n;
      while ((n = walkerAll.nextNode())) allNodes.push(n);

      // Find text nodes that overlap with the selection range and record
      // the selected portion of each one.
      var selected = []; // { node, text, startOff }
      for (var i = 0; i < allNodes.length; i++) {
        var node = allNodes[i];
        var nodeRange = document.createRange();
        nodeRange.selectNodeContents(node);

        // Node ends before selection starts — skip.
        if (range.compareBoundaryPoints(Range.END_TO_START, nodeRange) >= 0) continue;
        // Node starts after selection ends — stop.
        if (range.compareBoundaryPoints(Range.START_TO_END, nodeRange) <= 0) break;

        var startOff = (node === range.startContainer) ? range.startOffset : 0;
        var endOff   = (node === range.endContainer)   ? range.endOffset   : node.nodeValue.length;
        var text = node.nodeValue.slice(startOff, endOff);
        if (text) selected.push({ node: node, text: text, startOff: startOff });
      }

      if (selected.length === 0) return null;

      // Each text node's contribution is its own segment. This ensures that selections
      // crossing inline elements (e.g. <a>, <strong>) remain findable — concatenating
      // across text nodes produces a string that exists in no single text node, breaking
      // indexOf-based highlight application. Whitespace-only nodes (e.g. newlines between
      // block elements) are filtered out.
      var segments = selected
        .filter(function(s) { return s.text.trim() !== ''; })
        .map(function(s) { return { text: s.text, startNode: s.node, startOff: s.startOff }; });

      if (segments.length === 0) return null;

      // For each segment, count how many times its text appeared in the document
      // before the occurrence that is part of our selection.
      // If the segment text spans multiple DOM nodes (inline elements), it won't be
      // found in any single text node — fall back to occurrence 0, which is correct
      // for virtually all real-world cross-inline-element selections.
      var texts = [];
      var occurrences = [];

      for (var si = 0; si < segments.length; si++) {
        var seg = segments[si];
        texts.push(seg.text);

        var count = 0;
        var found = false;
        for (var i = 0; i < allNodes.length && !found; i++) {
          var node = allNodes[i];
          var nodeText = node.nodeValue;
          var start = 0;
          var idx;
          while ((idx = nodeText.indexOf(seg.text, start)) !== -1) {
            if (node === seg.startNode && idx === seg.startOff) {
              found = true;
              break;
            }
            count++;
            start = idx + seg.text.length;
          }
        }
        occurrences.push(count); // 0 when not found (cross-node segment)
      }

      return { texts: texts, occurrences: occurrences };
    }

    // ── Init: apply stored highlights ──────────────────────────────────────────

    document.addEventListener('DOMContentLoaded', function() {
      __ANNOTATIONS__.forEach(applyHighlight);
    });

    // ── Selection detection ────────────────────────────────────────────────────

    document.addEventListener('mouseup', function(e) {
      // If the click landed on a highlight mark, let the mark's own click handler
      // post HIGHLIGHT_CLICKED. Suppressing SELECTION_CLEARED here prevents the
      // tooltip from being opened and immediately dismissed.
      if (e.target && e.target.closest && e.target.closest('mark[data-ann-id]')) return;
      setTimeout(function() {
        var sel = window.getSelection();
        if (!sel || sel.isCollapsed || !sel.toString().trim()) {
          window.parent.postMessage({ type: '${SELECTION_CLEARED}' }, '*');
          return;
        }
        var result = getSegmentsAndOccurrences();
        if (!result) {
          window.parent.postMessage({ type: '${SELECTION_CLEARED}' }, '*');
          return;
        }
        var range = sel.getRangeAt(0);
        var rect = range.getBoundingClientRect();
        window.parent.postMessage({
          type: '${SELECTION_MADE}',
          texts: result.texts,
          occurrences: result.occurrences,
          rect: { top: rect.top, left: rect.left, width: rect.width, height: rect.height }
        }, '*');
      }, 10);
    });

    // ── Find-in-page ───────────────────────────────────────────────────────────

    var __findMatches__ = [];
    var __findCurrent__ = -1;

    function clearFindMarks() {
      var marks = document.querySelectorAll('mark.xf');
      for (var i = 0; i < marks.length; i++) {
        var m = marks[i];
        var p = m.parentNode;
        while (m.firstChild) p.insertBefore(m.firstChild, m);
        p.removeChild(m);
      }
      document.body.normalize();
      __findMatches__ = [];
      __findCurrent__ = -1;
    }

    function findInPage(query) {
      clearFindMarks();
      if (!query) {
        window.parent.postMessage({ type: '${FIND_RESULT}', total: 0, current: 0 }, '*');
        return;
      }
      var lower = query.toLowerCase();
      var walker = makeTextWalker(document.body);
      var node;
      var collected = [];
      while ((node = walker.nextNode())) {
        var text = node.nodeValue.toLowerCase();
        var start = 0;
        var idx;
        while ((idx = text.indexOf(lower, start)) !== -1) {
          collected.push({ node: node, start: idx, end: idx + query.length });
          start = idx + lower.length;
        }
      }
      // Apply marks in reverse so earlier ranges stay valid after DOM mutations.
      var markEls = [];
      for (var i = collected.length - 1; i >= 0; i--) {
        try {
          var range = document.createRange();
          range.setStart(collected[i].node, collected[i].start);
          range.setEnd(collected[i].node, collected[i].end);
          var mark = document.createElement('mark');
          mark.className = 'xf';
          range.surroundContents(mark);
          markEls.unshift(mark);
        } catch (e) {}
      }
      __findMatches__ = markEls;
      if (__findMatches__.length > 0) {
        __findCurrent__ = 0;
        __findMatches__[0].className = 'xf xfa';
        __findMatches__[0].scrollIntoView({ block: 'center', behavior: 'smooth' });
      }
      window.parent.postMessage({
        type: '${FIND_RESULT}',
        total: __findMatches__.length,
        current: __findMatches__.length > 0 ? 1 : 0
      }, '*');
    }

    function navigateFind(direction) {
      if (__findMatches__.length === 0) return;
      __findMatches__[__findCurrent__].className = 'xf';
      __findCurrent__ = (__findCurrent__ + direction + __findMatches__.length) % __findMatches__.length;
      __findMatches__[__findCurrent__].className = 'xf xfa';
      __findMatches__[__findCurrent__].scrollIntoView({ block: 'center', behavior: 'smooth' });
      window.parent.postMessage({
        type: '${FIND_RESULT}',
        total: __findMatches__.length,
        current: __findCurrent__ + 1
      }, '*');
    }

    document.addEventListener('keydown', function(e) {
      if ((e.metaKey || e.ctrlKey) && e.key === 'f') {
        e.preventDefault();
        window.parent.postMessage({ type: '${FIND_OPEN}' }, '*');
      } else if (e.key === 'Escape') {
        window.parent.postMessage({ type: '${FIND_CLEAR}' }, '*');
      }
    });

    // ── Link interception (existing) ───────────────────────────────────────────

    document.addEventListener('click', function(e) {
      var a = e.target.closest('a[href]');
      if (a) {
        e.preventDefault();
        window.parent.postMessage({ type: '${OPEN_LINK_URL}', url: a.href }, '*');
      }
    });

    // ── Parent → iframe messages ───────────────────────────────────────────────

    window.addEventListener('message', function(e) {
      var msg = e.data;
      if (!msg || !msg.type) return;
      if (msg.type === 'apply-highlights') {
        msg.annotations.forEach(applyHighlight);
        var sel = window.getSelection();
        if (sel) sel.removeAllRanges();
      } else if (msg.type === 'remove-highlight') {
        removeHighlight(msg.annotationId);
      } else if (msg.type === '${FIND_IN_PAGE}') {
        findInPage(msg.query || '');
      } else if (msg.type === '${FIND_NAVIGATE}') {
        navigateFind(msg.direction);
      } else if (msg.type === '${FIND_CLEAR}') {
        clearFindMarks();
        window.parent.postMessage({ type: '${FIND_RESULT}', total: 0, current: 0 }, '*');
      } else if (msg.type === '${SCROLL_TO_ANNOTATION}') {
        var mark = document.querySelector('mark[data-ann-id="' + msg.annotationId + '"]');
        if (mark) mark.scrollIntoView({ block: 'center', behavior: 'smooth' });
      }
    });
  </script></body></html>`;
}
