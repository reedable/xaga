import { Typography } from "@mui/material";
import { isEntry, type Item } from "../../types";
import cx from "../../utils/cx";
import ArticleReader from "./ArticleReader";
import css from "./_.module.css";

interface ItemViewProps {
  className?: string;
  selectedItem: Item | null; // FIXME Should this accept Item type?
  onSelectRelatedItem: (item: Item) => void;
  onChangeItem?: (item: Item) => void;
}

export function ItemView({
  className,
  selectedItem,
  onSelectRelatedItem,
  onChangeItem,
}: ItemViewProps) {
  if (!selectedItem) {
    return (
      <div className={cx(className, css.Placeholder)}>
        <Typography variant="body2" color="text.disabled">
          Select an article
        </Typography>
      </div>
    );
  }

  return (
    <div className={cx(className, css.LayoutContainer)}>
      <div className={css.Layout}>
        {isEntry(selectedItem) && (
          <ArticleReader
            selectedEntry={selectedItem}
            onChangeEntry={onChangeItem}
            onSelectEntry={onSelectRelatedItem}
          />
        )}
      </div>
    </div>
  );
}
