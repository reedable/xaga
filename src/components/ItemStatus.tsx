import { Bookmark, Circle } from "@mui/icons-material";
import { isEntry, type Item } from "../types";
import { Byline } from "./Byline";
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
      {isEntry(item) && item.isBookmarked && <Bookmark color="primary" />}
    </span>
  );
}
