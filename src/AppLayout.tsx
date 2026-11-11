import {
  Alert,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  Snackbar,
} from "@mui/material";
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
} from "react";
import { useStartupFeedRefresh } from "./hooks/useStartupFeedRefresh";
import css from "./AppLayout.module.css";
import { EntityView } from "./components/content/EntityView";
import EditSiteDialog from "./components/dialogs/EditSiteDialog";
import EditFeedDialog from "./components/dialogs/EditFeedDialog";
import EditTopicDialog from "./components/dialogs/EditTopicDialog";
import { EntityList } from "./components/list/EntityList";
import { ColumnResizeHandle } from "./components/ColumnResizeHandle";
import { SearchPanel } from "./components/search/SearchPanel";
import { SearchResultList } from "./components/search/SearchResultList";
import { Sidebar } from "./components/sidebar/Sidebar";
import { TitleBar } from "./components/titlebar/TitleBar";
import ShortcutsMenu from "./components/ShortcutsMenu";
import useAbortController from "./hooks/useAbortController";
import { useAppearance } from "./hooks/useAppearance";
import { useFeeds } from "./hooks/useFeeds";
import useRouter from "./hooks/useRouter";
import { useTopics } from "./hooks/useTopics";
import { usePlatform } from "./platform/Platform";
import { useEmbeddingStartupState } from "./hooks/useEmbeddingStartupState";
import { useSettings } from "./hooks/useSettings";
import { useSidebarFilter } from "./hooks/useSidebarFilter";
import { type Feed, type Topic, type Site, type Filter } from "./types";

export function AppLayout({
  libraryBadgeRef,
}: {
  libraryBadgeRef?: (element: HTMLSpanElement | null) => void;
}) {
  useStartupFeedRefresh();
  const { selectTopic, selectFeed, search, clearSearch } = useRouter();
  const [listFilter, setListFilter] = useState<Filter>("unread");
  const appRef = useRef<HTMLDivElement>(null);
  const readingSpaceRef = useRef<HTMLDivElement>(null);
  const [sidebarWidth, setSidebarWidth] = useState(15);
  const [listWidth, setListWidth] = useState(20);
  const [searchDrawer, setSearchDrawer] = useState(false);
  const [autoHideSidebar, setAutoHideSidebar] = useState(false);
  const { showSidebar } = useAppearance();
  const { library } = useSettings();
  const embeddingStartupState = useEmbeddingStartupState();
  const [sidebarFilter, setSidebarFilter] = useSidebarFilter(library);

  const platform = usePlatform();
  const [clearHistoryOpen, setClearHistoryOpen] = useState(false);

  const [, abortControllerRef] = useAbortController();
  const [editingTopic, setEditingTopic] = useState<Topic | null>(null);
  const [isAddingTopic, setIsAddingTopic] = useState(false);
  const [addTopicTitle, setAddTopicTitle] = useState("");
  const [editingSite, setEditingSite] = useState<Site | null>(null);
  const [editingFeed, setEditingFeed] = useState<Feed | null>(null);
  const [isAddingFeed, setIsAddingFeed] = useState(false);

  const { requestTopics, createTopic, saveTopic, deleteTopic, setTopicPinned } =
    useTopics(abortControllerRef);

  const { requestFeeds, addFeed, renameFeed, deleteFeed, setFeedPinned } =
    useFeeds(abortControllerRef);

  const closeEditTopic = useCallback(() => {
    setEditingTopic(null);
    setIsAddingTopic(false);
    setAddTopicTitle("");
  }, []);

  const handleSaveSearchAsTopic = useCallback((title: string) => {
    setAddTopicTitle(title);
    setIsAddingTopic(true);
  }, []);

  const handleSaveTopic = useCallback(
    async (
      title: string,
      description: string,
      threshold: number,
      pinned: boolean
    ) => {
      let topic: Topic | undefined;

      if (editingTopic) {
        topic = await saveTopic({
          ...editingTopic,
          title,
          description,
          threshold,
        });
      } else {
        topic = await createTopic({ title, description, threshold });
      }

      if (topic) {
        setTopicPinned(topic.id, pinned);

        await requestTopics();
        setListFilter("unread");
        selectTopic(topic);
      }
    },
    [
      editingTopic,
      saveTopic,
      createTopic,
      setTopicPinned,
      requestTopics,
      selectTopic,
    ]
  );

  const handleDeleteTopic = useCallback(
    async (topic: Topic) => {
      await deleteTopic(topic.id);
      // Pin lived in the topic `.md`; deleting the file removes it. No separate cleanup.
      await requestTopics();
    },
    [deleteTopic, requestTopics]
  );

  const closeEditFeed = useCallback(() => {
    setEditingFeed(null);
    setIsAddingFeed(false);
  }, []);

  const handleAddFeed = useCallback(
    async (url: string, title: string, pinned: boolean) => {
      const feed = await addFeed(url);

      if (feed) {
        if (title) await renameFeed(feed.id, title);
        if (pinned) setFeedPinned(feed.id, true);
        await requestFeeds();
        selectFeed(feed);
      }
    },
    [addFeed, renameFeed, setFeedPinned, requestFeeds, selectFeed]
  );

  const handleSaveFeed = useCallback(
    async (title: string, pinned: boolean) => {
      if (!editingFeed) return;
      await renameFeed(editingFeed.id, title);

      setFeedPinned(editingFeed.id, pinned);

      await requestFeeds();
    },
    [editingFeed, renameFeed, setFeedPinned, requestFeeds]
  );

  const handleDeleteFeed = useCallback(
    async (feed: { url: string; id: string }) => {
      await deleteFeed(feed.url);
      // Pin lived in the feed `.atom`; deleting the file removes it. No separate cleanup.
      await requestFeeds();
    },
    [deleteFeed, requestFeeds]
  );

  useEffect(() => {
    const el = appRef.current;
    if (!el) return;
    const observer = new ResizeObserver(([entry]) => {
      const threshold = search !== undefined ? 1300 : 980;
      setAutoHideSidebar(entry.contentRect.width < threshold);
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, [search]);

  useLayoutEffect(() => {
    const element = readingSpaceRef.current;
    if (!element) return;
    const measure = () => {
      const rootSize = parseFloat(
        getComputedStyle(document.documentElement).fontSize
      );
      const style = getComputedStyle(element);
      const available =
        element.getBoundingClientRect().width -
        parseFloat(style.borderLeftWidth) -
        parseFloat(style.borderRightWidth);
      setSearchDrawer(available < 38 * rootSize);
    };
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    observer.observe(document.documentElement);
    measure();
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    requestFeeds();
  }, [requestFeeds]);

  useEffect(() => {
    requestTopics();
  }, [requestTopics]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    platform
      .onClearHistory(() => setClearHistoryOpen(true))
      .then((fn) => {
        unlisten = fn;
      });
    return () => unlisten?.();
  }, [platform]);

  return (
    <div className={css.Root}>
      <ShortcutsMenu />
      <TitleBar className={css.TitleBar} />

      <Snackbar
        open={embeddingStartupState === "unavailable"}
        anchorOrigin={{ vertical: "top", horizontal: "center" }}
      >
        <Alert severity="warning" variant="filled">
          Embedding features are unavailable. Search and related items are
          disabled. Restart Xaga to retry.
        </Alert>
      </Snackbar>

      <div
        ref={appRef}
        className={css.App}
        style={
          {
            "--sidebar-width": `${sidebarWidth}rem`,
            "--list-width": `${listWidth}rem`,
          } as CSSProperties
        }
      >
        {showSidebar && !autoHideSidebar && (
          <div id="sidebar-column" className={css.SidebarColumn}>
            <Sidebar
              libraryBadgeRef={libraryBadgeRef}
              filter={sidebarFilter}
              onFilterChange={setSidebarFilter}
              onAddFeed={() => setIsAddingFeed(true)}
              onAddTopic={() => setIsAddingTopic(true)}
            />
            <ColumnResizeHandle
              name="Sidebar width"
              target="sidebar-column"
              value={sidebarWidth}
              min={15}
              max={22.5}
              onChange={setSidebarWidth}
            />
          </div>
        )}

        <div id="entity-list-column" className={css.ListColumn}>
          <EntityList
            filter={listFilter}
            onFilterChange={setListFilter}
            className={css.EntityList}
            isSelectableHeader={autoHideSidebar || !showSidebar}
            onEditFeed={setEditingFeed}
            onEditTopic={setEditingTopic}
            onEditSite={setEditingSite}
          />

          <ColumnResizeHandle
            name="Article list width"
            target="entity-list-column"
            value={listWidth}
            min={20}
            max={30}
            onChange={setListWidth}
          />
        </div>
        <div
          ref={readingSpaceRef}
          className={css.ReadingSpace}
          data-testid="reading-space"
        >
          <EntityView className={css.EntityView} />
          {search !== undefined && (
            <SearchPanel drawer={searchDrawer} onClose={clearSearch}>
              <SearchResultList
                className={css.SearchResultList}
                searchQuery={search}
                onSaveAsTopic={handleSaveSearchAsTopic}
              />
            </SearchPanel>
          )}
        </div>
      </div>

      <EditTopicDialog
        open={editingTopic !== null || isAddingTopic}
        topic={editingTopic}
        initialTitle={addTopicTitle}
        onClose={closeEditTopic}
        onSave={handleSaveTopic}
        onDelete={handleDeleteTopic}
      />

      <EditSiteDialog site={editingSite} onClose={() => setEditingSite(null)} />

      <EditFeedDialog
        open={editingFeed !== null || isAddingFeed}
        feed={editingFeed}
        onClose={closeEditFeed}
        onAdd={handleAddFeed}
        onSave={handleSaveFeed}
        onDelete={handleDeleteFeed}
      />

      <Dialog
        open={clearHistoryOpen}
        onClose={() => setClearHistoryOpen(false)}
      >
        <DialogTitle>Clear History</DialogTitle>
        <DialogContent>
          <DialogContentText>
            This will clear all cookies and website data from external sites
            visited in Xaga. Xaga&apos;s feeds, entries, and settings will not
            be affected.
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button
            data-testid="cancel"
            onClick={() => setClearHistoryOpen(false)}
          >
            Cancel
          </Button>
          <Button
            color="error"
            onClick={async () => {
              await platform.clearWebviewHistory();
              setClearHistoryOpen(false);
            }}
          >
            Clear History
          </Button>
        </DialogActions>
      </Dialog>
    </div>
  );
}
