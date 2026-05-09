import { RssFeed } from "@mui/icons-material";
import {
  CircularProgress,
  ListItem,
  ListItemButton,
  ListItemIcon,
  ListItemText,
} from "@mui/material";
import { motion } from "framer-motion";
import type { ElementType, ReactNode } from "react";
import type { Collection } from "../../types";
import cx from "../../utils/cx";

const MotionListItem = motion(ListItem);

export interface CollectionListItemProps<T extends Collection> {
  className?: string;
  collection: T;
  icon?: ElementType;
  loading?: boolean;
  selected?: boolean;
  onSelectCollection: (collection: T) => void;
  renderSecondaryAction?: (collection: Collection) => ReactNode;
}

export default function CollectionListItem<T extends Collection>({
  className,
  collection,
  icon: Icon = RssFeed,
  loading = false,
  selected = false,
  onSelectCollection,
  renderSecondaryAction,
}: CollectionListItemProps<T>) {
  return (
    <MotionListItem
      layout
      transition={{ duration: 0.3, ease: "easeOut" }}
      disablePadding
      secondaryAction={renderSecondaryAction?.(collection)}
    >
      <ListItemButton
        className={cx(className)}
        selected={selected}
        onClick={() => onSelectCollection(collection)}
      >
        <ListItemIcon sx={{ minWidth: 32 }}>
          {loading ? (
            <CircularProgress size={16} />
          ) : (
            <Icon sx={{ fontSize: 18, color: "text.secondary" }} />
          )}
        </ListItemIcon>

        <ListItemText
          primary={collection.title}
          primaryTypographyProps={{
            component: "div",
            variant: "body2",
            noWrap: true,
          }}
        />
      </ListItemButton>
    </MotionListItem>
  );
}
