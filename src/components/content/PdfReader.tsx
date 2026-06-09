import { useEffect, useState } from "react";
import cx from "../../utils/cx";
import css from "./PdfReader.module.css";

export interface PdfReaderProps {
  /**
   * PDF bytes fetched and validated in Rust (through the SSRF proxy, `%PDF-`
   * checked). Null while the content loader is still resolving the URL.
   */
  bytes: Uint8Array<ArrayBuffer> | null;
  className?: string;
}

export function PdfReader({ bytes, className }: PdfReaderProps) {
  const [blobUrl, setBlobUrl] = useState("");

  useEffect(() => {
    if (!bytes) return;

    const url = URL.createObjectURL(
      new Blob([bytes], { type: "application/pdf" })
    );

    (async () => {
      setBlobUrl((prev) => {
        URL.revokeObjectURL(prev);
        return url;
      });
    })();
  }, [bytes]);

  useEffect(() => {
    return () => URL.revokeObjectURL(blobUrl);
  }, [blobUrl]);

  return (
    <div className={cx(css.PdfReader, className)}>
      <iframe src={blobUrl} title="PDF viewer" />
    </div>
  );
}
