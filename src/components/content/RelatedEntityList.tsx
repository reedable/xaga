import {
  List,
  ListItem,
  ListItemButton,
  ListItemText,
} from "@mui/material";
import { useCallback, useEffect, useState } from "react";
import { RELATED_THRESHOLD } from "../../consts";
import useAbortController from "../../hooks/useAbortController";
import { useEvents } from "../../hooks/useEvents";
import useRelatedEntities from "../../hooks/useRelatedEntities";
import useRouter from "../../hooks/useRouter";
import { useEmbeddingStartupState } from "../../hooks/useEmbeddingStartupState";
import { isPartialAnnotation, type Ranked, type ManagedEntity } from "../../types";
import cx from "../../utils/cx";
import { abortable, decorate, retriable } from "../../utils/Decorator";
import Logger from "../../utils/Logger";
import EntityStatus from "../EntityStatus";
import Quotes from "../Quotes";
import css from "./RelatedEntityList.module.css";

interface RelatedEntityListProps {
  className?: string;
  entityId: ManagedEntity["id"];
}

export function RelatedEntityList({ className, entityId }: RelatedEntityListProps) {
  const embeddingStartupState = useEmbeddingStartupState();
  const { selectEntity } = useRouter();
  const [abort, abortControllerRef] = useAbortController();
  const { queryByEntity } = useRelatedEntities(abortControllerRef);
  const [embeddedEntities, setEmbeddedEntities] = useState<Ranked<ManagedEntity>[]>([]);
  const {
    onUnreadChanged,
    onBookmarkChanged,
    onAnnotationChanged,
    onAnnotationReplaced,
  } = useEvents();

  const requestRelatedEntities = useCallback(
    async (id: string) => {
      setEmbeddedEntities([]);

      const retriable_queryByEntity = decorate(async () => {
        const results = await queryByEntity(id, "all", RELATED_THRESHOLD, 1, 12);
        if (!results) throw new Error("query failed");
        return results;
      })
        .with(retriable, {
          delay: 400,
          attempts: 6,
          logger: Logger.getLogger("retriable_queryByEntity"),
        })
        .with(abortable, { signal: abortControllerRef.current.signal });

      const results = await retriable_queryByEntity();
      if (results) setEmbeddedEntities(results);
    },
    [abortControllerRef, queryByEntity]
  );

  const handleSelectEntity = useCallback(
    (entity: ManagedEntity) => selectEntity(entity),
    [selectEntity]
  );

  useEffect(() => {
    if (embeddingStartupState !== "available") return;
    abort();

    (async () => {
      await requestRelatedEntities(entityId);
    })();

    return abort;
  }, [abort, requestRelatedEntities, entityId, embeddingStartupState]);

  useEffect(() => {
    return onUnreadChanged((entry) => {
      setEmbeddedEntities((prev) =>
        prev.map((p) => (p.id === entry.id ? { ...p, ...entry } : p))
      );
    });
  }, [onUnreadChanged]);

  useEffect(() => {
    return onBookmarkChanged((entry) => {
      setEmbeddedEntities((prev) =>
        prev.map((p) => (p.id === entry.id ? { ...p, ...entry } : p))
      );
    });
  }, [onBookmarkChanged]);

  useEffect(() => {
    return onAnnotationChanged((entity) => {
      setEmbeddedEntities((prev) =>
        prev.map((p) => (p.id === entity.id ? { ...p, color: entity.color } : p))
      );
    });
  }, [onAnnotationChanged]);

  useEffect(() => {
    return onAnnotationReplaced(({ oldId, entity }) => {
      setEmbeddedEntities((prev) =>
        prev.map((value) =>
          value.id === oldId ? { ...value, ...entity } : value
        )
      );
    });
  }, [onAnnotationReplaced]);

  if (embeddingStartupState !== "available" || embeddedEntities.length === 0)
    return null;

  return (
    <div className={cx(className, css.RelatedEntityList)}>
      <List className={css.List}>
        {embeddedEntities.map((entity) => (
          <ListItem key={entity.id} className={css.ListEntity} disablePadding>
            <ListItemButton
              className={css.ListItemButton}
              onClick={() => handleSelectEntity(entity)}
            >
              <ListItemText
                primary={
                  <Quotes
                    quoted={isPartialAnnotation(entity)}
                    color={isPartialAnnotation(entity) ? entity.color : undefined}
                  >
                    {entity.title}
                  </Quotes>
                }
                secondary={<EntityStatus entity={entity} />}
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
