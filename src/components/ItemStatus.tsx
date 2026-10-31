import { Bookmark, Circle } from "@mui/icons-material";
import { isEntry, type Item } from "../types";
import { Byline } from "./Byline";
import colorCss from "../colors.module.css";
import css from "./ItemStatus.module.css";

interface ItemStatusProps {
  item: Item;
}

export default function ItemStatus({ item }: ItemStatusProps) {
  return (
    <span className={css.ItemStatus}>
      {isEntry(item) && item.isUnread && (
        <Circle className={css.UnreadIcon} color="primary" />
      )}
      <Byline className={css.Byline} item={item} />
      {isEntry(item) && item.color && (
        <Bookmark
          className={colorCss[item.color]}
          style={{ color: "var(--color-icon)" }}
        />
      )}
    </span>
  );
}
