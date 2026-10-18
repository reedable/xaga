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
import { useEvents } from "../../hooks/useEvents";
import useRelatedItems from "../../hooks/useRelatedItems";
import useRouter from "../../hooks/useRouter";
import { useEmbeddingStartupState } from "../../hooks/useEmbeddingStartupState";
import { isPartialAnnotation, type Ranked, type Item } from "../../types";
import cx from "../../utils/cx";
import { abortable, decorate, retriable } from "../../utils/Decorator";
import Logger from "../../utils/Logger";
import ItemStatus from "../ItemStatus";
import Quotes from "../Quotes";
import css from "./RelatedItemList.module.css";

interface RelatedItemListProps {
  className?: string;
  itemId: Item["id"];
}

export function RelatedItemList({ className, itemId }: RelatedItemListProps) {
  const embeddingStartupState = useEmbeddingStartupState();
  const { selectItem } = useRouter();
  const [abort, abortControllerRef] = useAbortController();
  const { queryByItem } = useRelatedItems(abortControllerRef);
  const [embeddedItems, setEmbeddedItems] = useState<Ranked<Item>[]>([]);
  const {
    onUnreadChanged,
    onBookmarkChanged,
    onAnnotationChanged,
    onAnnotationReplaced,
  } = useEvents();

  const requestRelatedItems = useCallback(
    async (id: string) => {
      setEmbeddedItems([]);

      const retriable_queryByItem = decorate(async () => {
        const results = await queryByItem(id, "all", RELATED_THRESHOLD, 1, 12);
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
    [abortControllerRef, queryByItem]
  );

  const handleSelectItem = useCallback(
    (item: Item) => selectItem(item),
    [selectItem]
  );

  useEffect(() => {
    if (embeddingStartupState !== "available") return;
    abort();

    (async () => {
      await requestRelatedItems(itemId);
    })();

    return abort;
  }, [abort, requestRelatedItems, itemId, embeddingStartupState]);

  useEffect(() => {
    return onUnreadChanged((entry) => {
      setEmbeddedItems((prev) =>
        prev.map((p) => (p.id === entry.id ? { ...p, ...entry } : p))
      );
    });
  }, [onUnreadChanged]);

  useEffect(() => {
    return onBookmarkChanged((entry) => {
      setEmbeddedItems((prev) =>
        prev.map((p) => (p.id === entry.id ? { ...p, ...entry } : p))
      );
    });
  }, [onBookmarkChanged]);

  useEffect(() => {
    return onAnnotationChanged((item) => {
      setEmbeddedItems((prev) =>
        prev.map((p) => (p.id === item.id ? { ...p, color: item.color } : p))
      );
    });
  }, [onAnnotationChanged]);

  useEffect(() => {
    return onAnnotationReplaced(({ oldId, item }) => {
      setEmbeddedItems((prev) =>
        prev.map((value) =>
          value.id === oldId ? { ...value, ...item } : value
        )
      );
    });
  }, [onAnnotationReplaced]);

  if (embeddingStartupState !== "available" || embeddedItems.length === 0)
    return null;

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
              onClick={() => handleSelectItem(item)}
            >
              <ListItemText
                primary={
                  <Quotes
                    quoted={isPartialAnnotation(item)}
                    color={isPartialAnnotation(item) ? item.color : undefined}
                  >
                    {item.title}
                  </Quotes>
                }
                secondary={<ItemStatus item={item} />}
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
