import cx from "../../utils/cx";
import css from "./PdfReader.module.css";

export interface PdfReaderProps {
  src: string;
  className?: string;
}

export function PdfReader({ src, className }: PdfReaderProps) {
  return (
    <div className={cx(css.PdfReader, className)}>
      {/* Deny scripts/plugins: a remote document served as the "PDF" could
          otherwise run script in this frame and postMessage the parent. The
          native PDF viewer renders without these privileges. */}
      <iframe src={src} title="PDF viewer" sandbox="" />
    </div>
  );
}
