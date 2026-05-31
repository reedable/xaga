import { Typography } from "@mui/material";
import { useCallback, useRef } from "react";
import useRouter from "../../hooks/useRouter";
import cx from "../../utils/cx";
import ArticleReader, { type ArticleReaderHandle } from "./ArticleReader";
import css from "./ItemView.module.css";
import { Toolbar } from "./Toolbar";
import Logger from "../../utils/Logger";

type ItemViewProps = {
  className?: string;
};

const logger = Logger.getLogger("ItemView");

export function ItemView({ className }: ItemViewProps) {
  const { state } = useRouter();
  const { entryId, url } = state;

  const articleReaderRef = useRef<ArticleReaderHandle>(null);

  const handleReload = useCallback(() => {
    logger.debug("handleReload", entryId, url, articleReaderRef.current);
    articleReaderRef.current?.reload();
  }, [entryId, url]);

  return (
    <div className={cx(className, css.LayoutContainer)}>
      <div className={css.Layout}>
        <Toolbar reload={entryId || url ? handleReload : null} />

        {entryId ? (
          <ArticleReader ref={articleReaderRef} />
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
