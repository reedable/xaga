import { useEffect, useRef } from "react";
import cx from "../../utils/cx";
import css from "./PdfReader.module.css";

export interface PdfReaderProps {
  /**
   * Blob URL of PDF bytes fetched and validated in Rust (through the SSRF
   * proxy, `%PDF-` checked). useContentLoader owns the blob's lifecycle.
   * Null while the content loader is still resolving the URL.
   */
  src: string | null;
  className?: string;
}

export function PdfReader({ src, className }: PdfReaderProps) {
  const iframeRef = useRef<HTMLIFrameElement>(null);

  // Navigate via location.replace, never by setting src: a src load leaves the
  // iframe an about:blank → document history pair that WebKit counts in the
  // webview's joint session history, hijacking Back/Forward from the router.
  useEffect(() => {
    if (!src) return;
    iframeRef.current?.contentWindow?.location.replace(src);
  }, [src]);

  return (
    <div className={cx(css.PdfReader, className)}>
      <iframe ref={iframeRef} title="PDF viewer" />
    </div>
  );
}
