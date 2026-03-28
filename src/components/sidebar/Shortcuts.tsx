import {
  Archive,
  Bookmark,
  Circle,
  Dns,
  InsertInvitation,
  Star,
} from "@mui/icons-material";
import { ListItemButton, ListItemIcon, ListItemText } from "@mui/material";
import { ALL_ITEMS, RECOMMENDED_ITEMS } from "../../Items";
import type { Filter, Item } from "../../types";
import cx from "../../utils/cx";
import css from "./_.module.css";

interface ShortcutsProps {
  className?: string;
  onSelectList: (item: Item, filter?: Filter) => void;
}

export function Shortcuts({ className, onSelectList }: ShortcutsProps) {
  return (
    <div className={cx(className)}>
      <ListItemButton
        className={css.Shortcut}
        onClick={() => onSelectList(RECOMMENDED_ITEMS, "unread")}
      >
        <ListItemIcon sx={{ minWidth: 32 }}>
          <Star className={css.Icon} />
        </ListItemIcon>

        <ListItemText
          primary="Recommended"
          slotProps={{ primary: { variant: "body2", noWrap: true } }}
        />
      </ListItemButton>

      <ListItemButton
        className={css.Shortcut}
        onClick={() => onSelectList(ALL_ITEMS, "unread")}
      >
        <ListItemIcon sx={{ minWidth: 32 }}>
          <Circle className={css.Icon} />
        </ListItemIcon>

        <ListItemText
          primary="Unread"
          slotProps={{ primary: { variant: "body2", noWrap: true } }}
        />
      </ListItemButton>

      <ListItemButton
        className={css.Shortcut}
        onClick={() => onSelectList(ALL_ITEMS, "bookmarked")}
      >
        <ListItemIcon sx={{ minWidth: 32 }}>
          <Bookmark className={css.Icon} />
        </ListItemIcon>

        <ListItemText
          primary="Bookmarked"
          slotProps={{ primary: { variant: "body2", noWrap: true } }}
        />
      </ListItemButton>

      <ListItemButton
        className={css.Shortcut}
        onClick={() => onSelectList(ALL_ITEMS, "upcoming")}
      >
        <ListItemIcon sx={{ minWidth: 32 }}>
          <InsertInvitation className={css.Icon} />
        </ListItemIcon>

        <ListItemText
          primary="Upcoming"
          slotProps={{ primary: { variant: "body2", noWrap: true } }}
        />
      </ListItemButton>

      <ListItemButton
        className={css.Shortcut}
        onClick={() => onSelectList(ALL_ITEMS, "all")}
      >
        <ListItemIcon sx={{ minWidth: 32 }}>
          <Dns className={css.Icon} />
        </ListItemIcon>

        <ListItemText
          primary="All items"
          slotProps={{ primary: { variant: "body2", noWrap: true } }}
        />
      </ListItemButton>

      <ListItemButton
        className={css.Shortcut}
        onClick={() => onSelectList(ALL_ITEMS, "archived")}
      >
        <ListItemIcon sx={{ minWidth: 32 }}>
          <Archive className={css.Icon} />
        </ListItemIcon>

        <ListItemText
          primary="Archived"
          slotProps={{ primary: { variant: "body2", noWrap: true } }}
        />
      </ListItemButton>
    </div>
  );
}
