import { RssFeed } from "@mui/icons-material";
import {
  CircularProgress,
  ListItem,
  ListItemButton,
  ListItemIcon,
  ListItemText,
} from "@mui/material";
import type { ElementType, ReactNode } from "react";
import type { Item } from "../../types";
import cx from "../../utils/cx";

export interface ExpanderItemProps {
  className?: string;
  icon?: ElementType;
  item: Item;
  loading?: boolean;
  selected?: boolean;
  onSelectItem: (item: Item) => void;
  renderSecondaryAction?: (item: Item) => ReactNode;
}

export default function ExpanderItem({
  className,
  icon: Icon = RssFeed,
  item,
  loading = false,
  selected = false,
  onSelectItem,
  renderSecondaryAction,
}: ExpanderItemProps) {
  return (
    <ListItem
      key={item.id}
      disablePadding
      secondaryAction={renderSecondaryAction?.(item)}
    >
      <ListItemButton
        className={cx(className)}
        selected={selected}
        onClick={() => onSelectItem(item)}
      >
        <ListItemIcon sx={{ minWidth: 32 }}>
          {loading ? (
            <CircularProgress size={16} />
          ) : (
            <Icon sx={{ fontSize: 18, color: "text.secondary" }} />
          )}
        </ListItemIcon>

        <ListItemText
          primary={item.title}
          primaryTypographyProps={{ variant: "body2", noWrap: true }}
        />
      </ListItemButton>
    </ListItem>
  );
}
