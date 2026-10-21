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
import cx from "../../utils/cx";
import css from "./SidebarItem.module.css";

const MotionListItem = motion(ListItem);

type SidebarSummary = {
  title: string;
  entryCount: number;
  unreadCount: number;
};

export interface SidebarItemProps<T extends SidebarSummary> {
  className?: string;
  collection: T;
  icon?: ElementType;
  loading?: boolean;
  selected?: boolean;
  onSelectCollection: (collection: T) => void;
  renderSecondaryAction?: (collection: T) => ReactNode;
}

export default function SidebarItem<T extends SidebarSummary>({
  className,
  collection,
  icon: Icon = RssFeed,
  loading = false,
  selected = false,
  onSelectCollection,
  renderSecondaryAction,
}: SidebarItemProps<T>) {
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
        <ListItemIcon className={css.ListItemIcon}>
          {loading ? (
            <CircularProgress size={16} />
          ) : (
            <Icon className={css.Icon} />
          )}
        </ListItemIcon>

        <ListItemText
          primary={collection.title}
          slotProps={{
            primary: {
              className: css.ListItemTextPrimary,
            },
          }}
        />
      </ListItemButton>
    </MotionListItem>
  );
}
