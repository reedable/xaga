import { WarningAmberOutlined } from "@mui/icons-material";
import {
  ListItem,
  ListItemButton,
  ListItemIcon,
  ListItemText,
} from "@mui/material";
import { useContext, useState } from "react";
import type { PinTarget } from "../../pins";
import SitesContext from "../../providers/SitesContext";
import TopicsContext from "../../providers/TopicsContext";
import FeedsContext from "../../providers/FeedsContext";
import MissingShortcutDialog from "./MissingShortcutDialog";
import css from "./ShortcutList.module.css";
import itemCss from "./SidebarEntity.module.css";

export default function MissingShortcut({ target }: { target: PinTarget }) {
  const [open, setOpen] = useState(false);
  const topics = useContext(TopicsContext);
  const sites = useContext(SitesContext);
  const feeds = useContext(FeedsContext);
  const pending =
    target.type === "site"
      ? sites?.hasLoaded === false
      : target.type === "topic"
        ? topics?.hasLoaded === false
        : target.type === "feed" && feeds?.hasLoaded === false;
  if (pending) {
    return (
      <ListItem
        aria-label="Loading shortcut"
        aria-busy="true"
        className={css.SkeletonRow}
      >
        <span aria-hidden="true" className={css.SkeletonIcon} />
        <span aria-hidden="true" className={css.SkeletonTitle} />
      </ListItem>
    );
  }
  return (
    <>
      <ListItem disablePadding>
        <ListItemButton
          data-testid="unavailable-shortcut"
          onClick={() => setOpen(true)}
        >
          <ListItemIcon className={itemCss.ListItemIcon}>
            <WarningAmberOutlined className={itemCss.Icon} />
          </ListItemIcon>
          <ListItemText
            primary="Shortcut unavailable"
            slotProps={{ primary: { className: itemCss.ListItemTextPrimary } }}
          />
        </ListItemButton>
      </ListItem>
      <MissingShortcutDialog
        target={target}
        open={open}
        onClose={() => setOpen(false)}
      />
    </>
  );
}
