import { useCallback, useEffect, useRef, useState } from "react";
import css from "./_.module.css";
import { ItemView } from "./components/content/ItemView";
import { ItemList } from "./components/list/ItemList";
import { Sidebar } from "./components/sidebar/Sidebar";
import { useFeeds } from "./hooks/useFeeds";
import { useSettings } from "./hooks/useSettings";
import { usePlatform } from "./platform/Platform";
import { BookmarksProvider } from "./providers/BookmarksProvider";
import { ALL_ITEMS } from "./Shortcuts";
import { isShortcut, type Filter, type Item } from "./types";

function App() {
  const platform = usePlatform();
  const { library, refreshOnStartup, settingsLoaded } = useSettings();
  const [selectedList, setSelectedList] = useState<Item>(ALL_ITEMS);
  const [selectedFilter, setSelectedFilter] = useState<Filter | undefined>();
  const [selectedItem, setSelectedItem] = useState<Item | null>(null);
  const [shuttingDown, setShuttingDown] = useState(false);

  const abortControllerRef = useRef(new AbortController());
  const { refreshAllFeeds } = useFeeds(abortControllerRef);

  const handleSelectList = useCallback((item: Item) => {
    setSelectedList(item);

    if (isShortcut(item)) {
      setSelectedFilter(item.filter);
    } else {
      setSelectedFilter("unread");
    }
  }, []);

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

      <BookmarksProvider>
        <div className={css.App}>
          <Sidebar
            className={css.Sidebar}
            selectedList={selectedList}
            onSelectList={handleSelectList}
          />

          <ItemList
            className={css.ItemList}
            selectedList={selectedList}
            selectedFilter={selectedFilter}
            onSelectItem={setSelectedItem}
          />

          <ItemView
            className={css.ItemView}
            selectedItem={selectedItem}
            onSelectRelatedItem={setSelectedItem}
            onChangeItem={handleChangeItem}
          />
        </div>
      </BookmarksProvider>
    </>
  );
}

export default App;
