import { RssFeed } from "@mui/icons-material";
import {
  CircularProgress,
  ListItem,
  ListItemButton,
  ListItemIcon,
  ListItemText,
  type ListItemProps,
} from "@mui/material";
import { motion } from "framer-motion";
import { forwardRef, type ElementType, type ReactNode } from "react";
import cx from "../../utils/cx";
import css from "./SidebarEntity.module.css";

const MotionListItem = motion(ListItem);
const DivListItem = forwardRef<HTMLDivElement, ListItemProps<"div">>(
  function DivListItem(props, ref) {
    return <ListItem {...props} component="div" ref={ref} />;
  }
);

type SidebarSummary = {
  title: string;
  entryCount: number;
  unreadCount: number;
};

export interface SidebarEntityProps<T extends SidebarSummary> {
  component?: "li" | "div";
  className?: string;
  testId?: string;
  collection: T;
  icon?: ElementType;
  loading?: boolean;
  selected?: boolean;
  onSelectCollection: (collection: T) => void;
  renderSecondaryAction?: (collection: T) => ReactNode;
}

export default function SidebarEntity<T extends SidebarSummary>({
  component = "li",
  className,
  testId,
  collection,
  icon: Icon = RssFeed,
  loading = false,
  selected = false,
  onSelectCollection,
  renderSecondaryAction,
}: SidebarEntityProps<T>) {
  // Virtuoso owns virtual row geometry; layout transforms would compete with it.
  const Row = component === "li" ? MotionListItem : DivListItem;
  return (
    <Row
      {...(component === "li"
        ? {
            layout: true,
            transition: { duration: 0.3, ease: "easeOut" as const },
          }
        : {})}
      disablePadding
      secondaryAction={renderSecondaryAction?.(collection)}
    >
      <ListItemButton
        data-testid={testId}
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
    </Row>
  );
}
