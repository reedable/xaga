import cx from "../../utils/cx";
import css from "./PdfReader.module.css";

export interface PdfReaderProps {
  src: string;
  className?: string;
}

export function PdfReader({ src, className }: PdfReaderProps) {
  return (
    <div className={cx(css.PdfReader, className)}>
      <iframe src={src} title="PDF viewer" />
    </div>
  );
}
