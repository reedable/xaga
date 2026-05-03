import { FormatQuote } from "@mui/icons-material";
import type { ReactNode } from "react";
import css from "./Quotes.module.css";

export interface QuotesProps {
  children: ReactNode;
  quoted?: boolean;
}

export default function Quotes({ children, quoted }: QuotesProps) {
  if (!quoted) return children;

  return (
    <span className={css.Quotes}>
      <FormatQuote sx={{ transform: "rotate(180deg)" }} />
      {children}
      <FormatQuote />
    </span>
  );
}
