import { Bookmark, Circle } from "@mui/icons-material";
import { isEntry, type ManagedEntity } from "../types";
import { Byline } from "./Byline";
import colorCss from "../colors.module.css";
import css from "./EntityStatus.module.css";

interface EntityStatusProps {
  entity: ManagedEntity;
  showBookmark?: boolean;
}

export default function EntityStatus({
  entity,
  showBookmark = true,
}: EntityStatusProps) {
  return (
    <span className={css.EntityStatus}>
      {isEntry(entity) && entity.isUnread && (
        <Circle className={css.UnreadIcon} color="primary" />
      )}
      <Byline className={css.Byline} entity={entity} />
      {showBookmark && isEntry(entity) && entity.color && (
        <Bookmark
          className={colorCss[entity.color]}
          style={{ color: "var(--color-icon)" }}
        />
      )}
    </span>
  );
}
