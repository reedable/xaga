import { Bookmark, Circle } from "@mui/icons-material";
import {
  List,
  ListItem,
  ListItemButton,
  ListItemText,
  Typography,
} from "@mui/material";
import { useCallback, useEffect, useState } from "react";
import { RELATED_THRESHOLD } from "../../consts";
import useAbortController from "../../hooks/useAbortController";
import { useBookmarks } from "../../hooks/useBookmarks";
import useRelatedItems from "../../hooks/useRelatedItems";
import { isEntry, type Embedded, type Item } from "../../types";
import cx from "../../utils/cx";
import { abortable, decorate, retriable } from "../../utils/Decorator";
import formatByline from "../../utils/formatByline";
import Logger from "../../utils/Logger";
import css from "./_.module.css";

interface RelatedItemListProps {
  className?: string;
  selectedItem: Item;
  onSelectItem: (item: Item) => void;
}

export function RelatedItemList({
  className,
  selectedItem,
  onSelectItem,
}: RelatedItemListProps) {
  const [abort, abortControllerRef] = useAbortController();
  const { queryByItem } = useRelatedItems(abortControllerRef);
  const [embeddedItems, setEmbeddedItems] = useState<(Embedded & Item)[]>([]);
  const { bookmarkMap } = useBookmarks(abortControllerRef);

  const requestRelatedItems = useCallback(
    async (item: Item) => {
      setEmbeddedItems([]);

      const retriable_queryByItem = decorate(async () => {
        const results = await queryByItem(
          item,
          "all",
          RELATED_THRESHOLD,
          1,
          12,
        );
        if (!results) throw new Error("query failed");
        return results;
      })
        .with(retriable, {
          delay: 400,
          attempts: 6,
          logger: Logger.getLogger("retriable_queryByItem"),
        })
        .with(abortable, { signal: abortControllerRef.current.signal });

      const results = await retriable_queryByItem();
      if (results) setEmbeddedItems(results);
    },
    [abortControllerRef, queryByItem],
  );

  useEffect(() => {
    abort();

    (async () => {
      requestRelatedItems(selectedItem);
    })();
  }, [abort, queryByItem, requestRelatedItems, selectedItem]);

  if (embeddedItems.length === 0) return null;

  return (
    <div className={cx(className, css.RelatedItemList)}>
      <div className={css.Header}>
        <Typography variant="overline" color="text.secondary">
          Related
        </Typography>
      </div>

      <List className={css.List}>
        {embeddedItems.map((item) => (
          <ListItem key={item.id} className={css.ListItem} disablePadding>
            <ListItemButton
              className={css.ListItemButton}
              onClick={() => onSelectItem(item)}
            >
              <ListItemText
                primary={item.title}
                secondary={
                  <>
                    {isEntry(item) && item.isUnread && (
                      <Circle className={css.UnreadIcon} />
                    )}

                    {bookmarkMap?.get(item.id) && <Bookmark />}

                    <span>{formatByline(item)}</span>
                  </>
                }
                slotProps={{
                  primary: { className: css.Primary },
                  secondary: { className: css.Secondary },
                }}
              />
            </ListItemButton>
          </ListItem>
        ))}
      </List>
    </div>
  );
}
