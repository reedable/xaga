import { Typography } from "@mui/material";
import { useRef } from "react";
import useRouter from "../../hooks/useRouter";
import cx from "../../utils/cx";
import ArticleReader, { type ArticleReaderHandle } from "./ArticleReader";
import css from "./ItemView.module.css";
import { Toolbar } from "./Toolbar";

type ItemViewProps = {
  className?: string;
};

export function ItemView({ className }: ItemViewProps) {
  const { state } = useRouter();
  const { entryId } = state;

  const readerRef = useRef<ArticleReaderHandle>(null);

  return (
    <div className={cx(className, css.LayoutContainer)}>
      <div className={css.Layout}>
        <Toolbar reload={entryId ? () => readerRef.current?.reload() : null} />

        {entryId ? (
          <ArticleReader ref={readerRef} />
        ) : (
          <div className={css.Placeholder}>
            <Typography variant="body2" color="text.disabled">
              Select an article
            </Typography>
          </div>
        )}
      </div>
    </div>
  );
}
