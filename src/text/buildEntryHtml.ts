import {
  FONT_FAMILY_CSS,
  READER_FONT_SIZE_DEFAULT,
  READER_LINE_HEIGHT_DEFAULT,
  type FontFamily,
  type ThemeMode,
} from "../providers/ViewContext";

export const OPEN_URL = "open-url";

export interface ReaderStyle {
  themeMode: ThemeMode;
  fontFamily: FontFamily;
  fontSize: number;
  lineHeight: number;
}

export function buildEntryHtml(bodyHtml: string, style: ReaderStyle): string {
  const { themeMode, fontFamily, fontSize, lineHeight } = style;
  const prefersDark = themeMode === "dark";
  const bg = prefersDark ? "#121212" : "#ffffff";
  const fg = prefersDark ? "#e0e0e0" : "#212121";
  const linkColor = prefersDark ? "#90caf9" : "#1565c0";
  const codeBg = prefersDark ? "#1e1e1e" : "#f5f5f5";
  const fontStack = FONT_FAMILY_CSS[fontFamily];

  return `<!DOCTYPE html><html><head><meta charset="utf-8"><style>
    *{box-sizing:border-box}
    body{margin:0 0 4.5rem 0;padding:0.5rem 2rem 2rem;background:${bg};color:${fg};font-family:${fontStack};font-size:${fontSize}px;line-height:${lineHeight}}
    article{max-width:680px;margin:0 auto}
    p,li{margin-bottom:1em}
    img{max-width:100%;height:auto}
    a{color:${linkColor};cursor:pointer}
    h1,h2,h3,h4,h5,h6{margin-top:1.5rem;margin-bottom:0.5rem}
    pre{overflow-x:auto;padding:1rem;background:${codeBg};border-radius:4px}
    code{background:${codeBg};padding:0.1em 0.3em;border-radius:3px}
    blockquote{border-left:4px solid #888;margin-left:0;padding-left:1rem;opacity:0.85}
    table{border-collapse:collapse;width:100%}
    th,td{border:1px solid #888;padding:0.4rem 0.6rem}
  </style></head><body><article>${bodyHtml}</article><script>
    document.addEventListener('click', function(e) {
      var a = e.target.closest('a[href]');
      if (a) { e.preventDefault(); window.parent.postMessage({ type: '${OPEN_URL}', url: a.href }, '*'); }
    });
  </script></body></html>`;
}

export function defaultReaderStyle(themeMode: ThemeMode): ReaderStyle {
  return {
    themeMode,
    fontFamily: "system",
    fontSize: READER_FONT_SIZE_DEFAULT,
    lineHeight: READER_LINE_HEIGHT_DEFAULT,
  };
}
