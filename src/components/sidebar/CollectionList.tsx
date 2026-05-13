import { Add, FolderOutlined } from "@mui/icons-material";
import { IconButton } from "@mui/material";
import { useCallback } from "react";
import { useSearchParams } from "react-router-dom";
import useRouter from "../../hooks/useRouter";
import { isFilter, type Collection } from "../../types";
import css from "./CollectionList.module.css";
import ExpandableList from "./ExpandableList";
import SidebarItem from "./SidebarItem";
import UnreadCount from "./UnreadCount";

type CollectionListProps = {
  isExpanded?: boolean;
  collections: Collection[];
  onAddCollection: () => void;
  onClickHeader: () => void;
};

export default function CollectionList({
  isExpanded = false,
  collections,
  onAddCollection,
  onClickHeader,
}: CollectionListProps) {
  const { navigate, state } = useRouter();
  const { collectionId } = state;
  const [searchParams] = useSearchParams();
  const filterParam = searchParams.get("filter");

  const handleSelectCollection = useCallback(
    (collection: Collection) => {
      const filter = isFilter(filterParam) ? filterParam : "all";
      navigate(`/?filter=${filter}`, {
        state: { collectionId: collection.id },
      });
    },
    [filterParam, navigate]
  );

  return (
    <ExpandableList
      label="Collections"
      isExpanded={isExpanded}
      onClickHeader={onClickHeader}
      renderHeaderAction={() => (
        <IconButton size="small" onClick={onAddCollection}>
          <Add className={css.AddIcon} />
        </IconButton>
      )}
    >
      {collections.map((collection) => (
        <SidebarItem
          collection={collection}
          key={collection.id}
          icon={FolderOutlined}
          selected={collectionId === collection.id}
          onSelectCollection={handleSelectCollection}
          renderSecondaryAction={() => (
            <UnreadCount count={collection.unreadCount} />
          )}
        />
      ))}
    </ExpandableList>
  );
}
