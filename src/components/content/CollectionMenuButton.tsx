import { CreateNewFolder } from "@mui/icons-material";
import {
  Badge,
  Checkbox,
  IconButton,
  ListItemText,
  Menu,
  MenuItem,
  Tooltip,
} from "@mui/material";
import { forwardRef, useCallback, useEffect, useImperativeHandle, useState } from "react";
import useAbortController from "../../hooks/useAbortController";
import { useCollections } from "../../hooks/useCollections";
import type { Annotation, Entry } from "../../types";
import css from "./CollectionMenuButton.module.css";

interface CollectionMenuButtonProps {
  item: Entry | Annotation | null;
}

export interface CollectionMenuButtonHandle {
  saveToCollections: (item: Entry | Annotation) => Promise<void>;
}

export const CollectionMenuButton = forwardRef<CollectionMenuButtonHandle, CollectionMenuButtonProps>(
  function CollectionMenuButton({ item }, ref) {
    const [, abortControllerRef] = useAbortController();
    const {
      collections,
      listEntryCollections,
      addCollectionItem,
      removeCollectionItem,
    } = useCollections(abortControllerRef);

    const [menuAnchor, setMenuAnchor] = useState<HTMLElement | null>(null);
    const [memberIds, setMemberIds] = useState<Set<string>>(new Set());
    const [selectedCollectionIds, setSelectedCollectionIds] = useState<Set<string>>(new Set());

    useEffect(() => {
      if (!item) return;
      listEntryCollections(item.id).then((ids) => {
        setMemberIds(new Set(ids ?? []));
      });
    }, [item, listEntryCollections]);

    useImperativeHandle(ref, () => ({
      async saveToCollections(item: Entry | Annotation) {
        await Promise.all(
          [...selectedCollectionIds].map((collectionId) => addCollectionItem(collectionId, item))
        );
        setSelectedCollectionIds(new Set());
      },
    }), [selectedCollectionIds, addCollectionItem]);

    const handleOpen = useCallback(
      async (e: React.MouseEvent<HTMLElement>) => {
        setMenuAnchor(e.currentTarget);
        if (item) {
          const ids = await listEntryCollections(item.id);
          setMemberIds(new Set(ids ?? []));
        }
      },
      [item, listEntryCollections]
    );

    const handleToggle = useCallback(
      async (collectionId: string) => {
        if (!item) {
          setSelectedCollectionIds((prev) => {
            const next = new Set(prev);
            if (next.has(collectionId)) next.delete(collectionId);
            else next.add(collectionId);
            return next;
          });
          return;
        }
        if (memberIds.has(collectionId)) {
          await removeCollectionItem(collectionId, item.id);
          setMemberIds((prev) => {
            const next = new Set(prev);
            next.delete(collectionId);
            return next;
          });
        } else {
          await addCollectionItem(collectionId, item);
          setMemberIds((prev) => new Set([...prev, collectionId]));
        }
      },
      [item, memberIds, addCollectionItem, removeCollectionItem]
    );

    return (
      <>
        <Tooltip title="Add to Collection">
          <span>
            <IconButton
              size="small"
              onClick={handleOpen}
              disabled={collections.length === 0}
            >
              <Badge
                badgeContent={item ? memberIds.size : selectedCollectionIds.size}
                color="primary"
                overlap="circular"
                sx={{
                  "& .MuiBadge-badge": {
                    fontSize: "0.6rem",
                    height: 13,
                    minWidth: 13,
                    padding: "0 2px",
                  },
                }}
              >
                <CreateNewFolder fontSize="small" />
              </Badge>
            </IconButton>
          </span>
        </Tooltip>

        <Menu
          anchorEl={menuAnchor}
          open={Boolean(menuAnchor)}
          onClose={() => setMenuAnchor(null)}
        >
          {collections.map((c) => (
            <MenuItem key={c.id} onClick={() => handleToggle(c.id)} dense>
              <Checkbox
                checked={item ? memberIds.has(c.id) : selectedCollectionIds.has(c.id)}
                size="small"
                disableRipple
                className={css.Checkbox}
              />

              <ListItemText primary={c.title} />
            </MenuItem>
          ))}
        </Menu>
      </>
    );
  }
);
