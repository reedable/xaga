import { Bookmark, Circle } from "@mui/icons-material";
import { Collapse } from "@mui/material";
import { isEntry, type Item } from "../types";
import { Byline } from "./Byline";
import css from './ItemStatus.module.css';

interface ItemStatusProps {
  item: Item;
};

export default function ItemStatus({ item }: ItemStatusProps) {
  return (
    <div className={css.ItemStatus}>
      {isEntry(item) && (
        <Collapse in={item.isUnread} orientation="horizontal">
          <Circle className={css.UnreadIcon} color="primary" />
        </Collapse>
      )}

      <Byline className={css.Byline} item={item} />

      {isEntry(item) && item.isBookmarked && (
        <Collapse in={item.isBookmarked} orientation="horizontal">
          <Bookmark color="primary" />
        </Collapse>
      )}
    </div>
  )
}