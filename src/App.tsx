import { useCallback, useEffect, useState } from "react";
import css from "./_.module.css";
import { ItemView } from "./components/content/ItemView";
import { ItemList } from "./components/list/ItemList";
import { Settings } from "./components/settings/Settings";
import { Sidebar } from "./components/sidebar/Sidebar";
import { ALL_ITEMS } from "./Items";
import { usePlatform } from "./platform/Platform";
import { BookmarksProvider } from "./providers/BookmarksProvider";
import FeedsProvider from "./providers/FeedsProvider";
import { isShortcut, type Filter, type Item } from "./types";
import Logger from "./utils/Logger";

const loggerContext = Logger.getLogger("App");

function App() {
  const platform = usePlatform();
  const [library, setLibrary] = useState<string>("");
  const [selectedList, setSelectedList] = useState<Item>(ALL_ITEMS);
  const [selectedFilter, setSelectedFilter] = useState<Filter | undefined>();
  const [selectedItem, setSelectedItem] = useState<Item | null>(null);
  const [shuttingDown, setShuttingDown] = useState(false);
  const [showSettings, setShowSettings] = useState(false);

  const handleOpenLibrary = useCallback(async () => {
    const selected = await platform.openFileDialog(true, false);

    if (typeof selected === "string") {
      const canonical = await platform.setLibrary(selected);
      setLibrary(canonical);
    }
  }, [platform]);

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
    platform.getLibrary().then(setLibrary).catch(loggerContext.error);
  }, [platform]);

  useEffect(() => {
    const name = library ? library.split("/").pop() || "Xaga" : null;
    const title = name ? `Xaga — ${name}` : "Xaga";
    platform.setWindowTitle(title);
  }, [library, platform]);

  useEffect(() => {
    const unlistens = [
      platform.onOpenLibrary(handleOpenLibrary),
      platform.onOpenSettings(() => setShowSettings(true)),
      platform.onShutdown(() => setShuttingDown(true)),
    ];

    return () => {
      unlistens.forEach((p) => p.then((fn: () => void) => fn()));
    };
  }, [handleOpenLibrary, platform]);

  return (
    <>
      {shuttingDown && (
        <div className={css.ShutdownOverlay}>Saving data, please wait…</div>
      )}

      {showSettings ? (
        <Settings onClose={() => setShowSettings(false)} />
      ) : (
        <FeedsProvider>
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
                onSelectArticle={setSelectedItem}
              />

              <ItemView
                className={css.ItemView}
                selectedItem={selectedItem}
                onSelectRelatedItem={setSelectedItem}
                onChangeItem={handleChangeItem}
              />
            </div>
          </BookmarksProvider>
        </FeedsProvider>
      )}
    </>
  );
}

export default App;
