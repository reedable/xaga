import { Typography } from "@mui/material";
import { useCallback, useState } from "react";
import useRouter from "../../hooks/useRouter";
import cx from "../../utils/cx";
import { type Article } from "../../types";
import ArticleReader from "./ArticleReader";
import css from "./ItemView.module.css";
import { Toolbar } from "./Toolbar";

type ItemViewProps = {
  className?: string;
};

export function ItemView({ className }: ItemViewProps) {
  const { state } = useRouter();
  const { entryId } = state;

  const [entry, setEntry] = useState<Article | null>(null);
  const [reload, setReload] = useState<(() => void) | null>(null);

  const handleReloadChange = useCallback((fn: (() => void) | null) => {
    setReload(fn ? () => fn : null);
  }, []);

  return (
    <div className={cx(className, css.LayoutContainer)}>
      <div className={css.Layout}>
        <Toolbar entry={entry} reload={reload} />
        {entryId ? (
          <ArticleReader
            onEntryChange={setEntry}
            onReloadChange={handleReloadChange}
          />
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
