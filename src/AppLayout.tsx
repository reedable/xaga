import { Collapse } from "@mui/material";
import { useCallback, useEffect, useMemo } from "react";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import css from "./_.module.css";
import { ItemView } from "./components/content/ItemView";
import { ItemList } from "./components/list/ItemList";
import { SearchResultList } from "./components/list/SearchResultList";
import { Sidebar } from "./components/sidebar/Sidebar";
import { TitleBar } from "./components/titlebar/TitleBar";
import { useManageFeed } from "./providers/ManageFeedProvider";
import { useManageTopic } from "./providers/ManageTopicProvider";
import { ALL_ITEMS, SHORTCUTS } from "./Shortcuts";
import { isFilter, isShortcut, type Collection } from "./types";

export function AppLayout() {
  const { collectionId, itemId } = useParams();
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();

  const { feeds } = useManageFeed();
  const { topics } = useManageTopic();

  const selectedCollection = useMemo(
    () =>
      SHORTCUTS.find((s) => s.id === collectionId) ??
      feeds.find((f) => f.id === collectionId) ??
      topics.find((t) => t.id === collectionId) ??
      ALL_ITEMS,
    [collectionId, feeds, topics],
  );

  const rawFilter = searchParams.get("filter");
  const filter = isFilter(rawFilter) ? rawFilter : undefined;
  const search = searchParams.get("search") ?? "";

  useEffect(() => {
    if (filter) localStorage.setItem("list.filter", filter);
  }, [filter]);

  const handleSelectCollection = useCallback(
    (collection: Collection) => {
      const newFilter = isShortcut(collection) ? collection.filter : "unread";
      navigate(`/${collection.id}?filter=${newFilter}`);
    },
    [navigate],
  );

  return (
    <div className={css.Root}>
      <TitleBar className={css.TitleBar} />

      <div className={css.App}>
        <Sidebar
          className={css.Sidebar}
          selectedCollection={selectedCollection}
          onSelectCollection={handleSelectCollection}
        />

        <ItemList
          className={css.ItemList}
          selectedCollection={selectedCollection}
        />

        <ItemView
          className={css.ItemView}
          itemId={itemId ?? null}
        />

        <Collapse in={!!search} orientation="horizontal">
          <SearchResultList
            className={css.SearchResultList}
            searchQuery={search}
          />
        </Collapse>
      </div>
    </div>
  );
}
