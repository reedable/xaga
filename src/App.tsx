import { useCallback, useEffect, useRef, useState } from "react";
import css from "./_.module.css";
import { ItemView } from "./components/content/ItemView";
import { ItemList } from "./components/list/ItemList";
import { SearchResultList } from "./components/list/SearchResultList";
import { Sidebar } from "./components/sidebar/Sidebar";
import { TitleBar } from "./components/titlebar/TitleBar";
import { useFeeds } from "./hooks/useFeeds";
import { useLocalStorage } from "./hooks/useLocalStorage";
import { useSettings } from "./hooks/useSettings";
import { usePlatform } from "./platform/Platform";
import { ALL_ITEMS } from "./Shortcuts";
import { isFeed, isShortcut, isTopic, type Feed, type Filter, type Item, type Topic } from "./types";
import { Collapse } from "@mui/material";

function App() {
  const platform = usePlatform();
  const { library, refreshOnStartup, settingsLoaded } = useSettings();
  const [selectedList, setSelectedList] = useState<Item>(ALL_ITEMS);
  const [selectedFilter, setSelectedFilter] = useLocalStorage<Filter | undefined>("list.filter", undefined);
  const [selectedItem, setSelectedItem] = useState<Item | null>(null);
  const [shuttingDown, setShuttingDown] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [editingFeed, setEditingFeed] = useState<Feed | null>(null);
  const [editingTopic, setEditingTopic] = useState<Topic | null>(null);

  const abortControllerRef = useRef(new AbortController());
  const { refreshAllFeeds } = useFeeds(abortControllerRef);

  const handleSelectList = useCallback((item: Item) => {
    setSelectedList(item);

    if (isShortcut(item)) {
      setSelectedFilter(item.filter);
    } else {
      setSelectedFilter("unread");
    }
  }, [setSelectedFilter]);

  const handleChangeItem = useCallback((item: Item) => {
    // TODO We call this when an article is marked read/unread in ItemView.
    //      Need to review the code and figure out the right way to handle it.
    console.log("handleChangeItem", item);
  }, []);

  useEffect(() => {
    const name = library ? library.split("/").pop() || "Xaga" : null;
    const title = name ? `Xaga — ${name}` : "Xaga";
    platform.setWindowTitle(title);
  }, [library, platform]);

  const didStartupRefresh = useRef(false);

  useEffect(() => {
    if (!settingsLoaded || didStartupRefresh.current) return;
    didStartupRefresh.current = true;
    if (refreshOnStartup) refreshAllFeeds();
  }, [settingsLoaded, refreshOnStartup, refreshAllFeeds]);

  useEffect(() => {
    const unlisten = platform.onShutdown(() => setShuttingDown(true));
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [platform]);

  return (
    <>
      {shuttingDown && (
        <div className={css.ShutdownOverlay}>Saving data, please wait…</div>
      )}

      <div className={css.Root}>
        <TitleBar
          className={css.TitleBar}
          onSearchQueryChange={setSearchQuery}
        />

        <div className={css.App}>
          <Sidebar
            className={css.Sidebar}
            selectedList={selectedList}
            onSelectList={handleSelectList}
            editingFeed={editingFeed}
            onSetEditingFeed={setEditingFeed}
            editingTopic={editingTopic}
            onSetEditingTopic={setEditingTopic}
          />

          <ItemList
            className={css.ItemList}
            selectedCollection={selectedList}
            selectedFilter={selectedFilter}
            onSelectItem={setSelectedItem}
            onFilterChange={setSelectedFilter}
            onEdit={
              isFeed(selectedList) ? () => setEditingFeed(selectedList) :
                isTopic(selectedList) ? () => setEditingTopic(selectedList) :
                  undefined
            }
          />

          <ItemView
            className={css.ItemView}
            selectedItem={selectedItem}
            onSelectRelatedItem={setSelectedItem}
            onChangeItem={handleChangeItem}
          />

          <Collapse in={!!searchQuery} orientation="horizontal">
            <SearchResultList
              className={css.SearchResultList}
              searchQuery={searchQuery}
              onSelectItem={setSelectedItem}
            />
          </Collapse>
        </div>
      </div>
    </>
  );
}

export default App;
