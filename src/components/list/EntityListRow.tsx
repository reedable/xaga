import { ListItemButton, ListItemText } from "@mui/material";
import { isEntry, isPartialAnnotation, type ManagedEntity } from "../../types";
import cx from "../../utils/cx";
import EntityStatus from "../EntityStatus";
import Quotes from "../Quotes";
import css from "./EntityList.module.css";
import colorCss from "../../colors.module.css";

type RowProps = {
  entity: ManagedEntity;
  selected: boolean;
  tabIndex: number;
  rowRef: React.Ref<HTMLDivElement>;
  onKeyDown: React.KeyboardEventHandler;
  onClick: () => void;
};
export function EntityListRow({
  entity,
  selected,
  tabIndex,
  rowRef,
  onKeyDown,
  onClick,
}: RowProps) {
  return (
    <div key={entity.id} className={css.ListEntity}>
      <ListItemButton
        data-bookmark-color={isEntry(entity) ? (entity.color ?? "") : undefined}
        ref={rowRef}
        className={cx(
          css.ListItemButton,
          isEntry(entity) && css.ArticleItem,
          isEntry(entity) && entity.color && css.Bookmarked,
          isEntry(entity) && entity.color && colorCss[entity.color]
        )}
        selected={selected}
        tabIndex={tabIndex}
        onKeyDown={onKeyDown}
        onClick={onClick}
      >
        <ListItemText
          primary={
            <>
              <Quotes
                quoted={isPartialAnnotation(entity)}
                color={isPartialAnnotation(entity) ? entity.color : undefined}
              >
                {entity.title}
              </Quotes>
              {isEntry(entity) && entity.color && (
                <span className={css.ScreenReaderOnly}>
                  {` Bookmarked, ${entity.color[0].toUpperCase()}${entity.color.slice(1)}`}
                </span>
              )}
            </>
          }
          secondary={<EntityStatus entity={entity} showBookmark={false} />}
          slotProps={{
            primary: { className: css.Primary },
            secondary: { className: css.Secondary },
          }}
        />
      </ListItemButton>
    </div>
  );
}
