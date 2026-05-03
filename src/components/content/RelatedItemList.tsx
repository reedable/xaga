import {
  List,
  ListItem,
  ListItemButton,
  ListItemText,
  Typography,
} from "@mui/material";
import { useCallback, useEffect, useState } from "react";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import { RELATED_THRESHOLD } from "../../consts";
import useAbortController from "../../hooks/useAbortController";
import { useEvents } from "../../hooks/useEvents";
import useRelatedItems from "../../hooks/useRelatedItems";
import { isAnnotation, type Embedded, type Item } from "../../types";
import cx from "../../utils/cx";
import { abortable, decorate, retriable } from "../../utils/Decorator";
import Logger from "../../utils/Logger";
import ItemStatus from "../ItemStatus";
import Quotes from "../Quotes";
import css from "./_.module.css";

interface RelatedItemListProps {
  className?: string;
  itemId: Item["id"];
}

export function RelatedItemList({ className, itemId }: RelatedItemListProps) {
  const navigate = useNavigate();
  const { collectionId } = useParams();
  const [searchParams] = useSearchParams();

  const [abort, abortControllerRef] = useAbortController();
  const { queryByItem } = useRelatedItems(abortControllerRef);
  const [embeddedItems, setEmbeddedItems] = useState<(Embedded & Item)[]>([]);
  const { onUnreadChanged } = useEvents();

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
    (item: Item) => {
      const params = new URLSearchParams(searchParams);
      navigate(`/${collectionId}/${item.id}?${params.toString()}`);
    },
    [navigate, collectionId, searchParams],
  );

  useEffect(() => {
    abort();
    (async () => { requestRelatedItems(itemId); })();
  }, [abort, queryByItem, requestRelatedItems, itemId]);

  useEffect(() => {
    return onUnreadChanged((entry) => {
      setEmbeddedItems((prev) =>
        prev.map((p) => (p.id === entry.id ? { ...p, ...entry } : p))
      );
    });
  }, [onUnreadChanged]);

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
              onClick={() => handleSelectItem(item)}
            >
              <ListItemText
                primary={
                  <Quotes quoted={isAnnotation(item)}>{item.title}</Quotes>
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
