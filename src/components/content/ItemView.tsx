import { Typography } from "@mui/material";
import { useCallback, useRef } from "react";
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
  const { entryId, url } = state;

  const readerRef = useRef<ArticleReaderHandle>(null);

  const handleReload = useCallback(() => {
    readerRef.current?.reload();
  }, []);

  return (
    <div className={cx(className, css.LayoutContainer)}>
      <div className={css.Layout}>
        <Toolbar reload={entryId || url ? handleReload : null} />

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
