import { useEffect, useRef } from "react";
import { OPEN_URL } from "../../text/buildEntryHtml";
import cx from "../../utils/cx";

interface ReaderProps {
  className?: string;
  html: string;
  onClickUrl?: (url: string) => void;
}

export function Reader({ className, html, onClickUrl }: ReaderProps) {
  const iframeRef = useRef<HTMLIFrameElement>(null);

  useEffect(() => {
    const blob = new Blob([html], { type: "text/html" });
    const url = URL.createObjectURL(blob);
    if (iframeRef.current) iframeRef.current.src = url;
    return () => URL.revokeObjectURL(url);
  }, [html]);

  useEffect(() => {
    function handleMessage(e: MessageEvent) {
      if (e.data?.type === OPEN_URL) {
        const url = e.data.url;
        if (url) onClickUrl?.(url);
      }
    }

    window.addEventListener("message", handleMessage);
    return () => window.removeEventListener("message", handleMessage);
  }, [onClickUrl]);

  return (
    <iframe
      ref={iframeRef}
      className={cx(className)}
      sandbox="allow-scripts"
      style={{
        width: "100%",
        height: "100%",
        border: "none",
        display: "block",
      }}
      title="Entry reader"
    />
  );
}
