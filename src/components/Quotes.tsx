import { FormatQuote } from "@mui/icons-material";
import type { ReactNode } from "react";
import colorCss from "../colors.module.css";
import cx from "../utils/cx";
import css from "./Quotes.module.css";

export interface QuotesProps {
  children: ReactNode;
  quoted?: boolean;
  color?: string | null;
}

export default function Quotes({ children, quoted, color }: QuotesProps) {
  if (!quoted) return children;

  const colorClass = color
    ? colorCss[color as keyof typeof colorCss]
    : undefined;

  return (
    <span className={cx(css.Quotes, colorClass)}>
      <FormatQuote className={css.Open} />
      {children}
      <FormatQuote />
    </span>
  );
}
