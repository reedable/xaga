import SidebarSortControl from "./SidebarSortControl";
import { Newspaper, Refresh } from "@mui/icons-material";
import { IconButton, List, MenuItem, TextField, Tooltip } from "@mui/material";
import { useCallback, useLayoutEffect, useMemo, useRef, useState } from "react";
import { ALL_ENTITIES } from "../../Shortcuts";
import { useAllEntitiesCounts } from "../../hooks/useAllEntitiesCounts";
import useRouter from "../../hooks/useRouter";
import SidebarEntity from "./SidebarEntity";
import UnreadCount from "./UnreadCount";
import useAbortController from "../../hooks/useAbortController";
import { useFeeds } from "../../hooks/useFeeds";
import { useLibraryReadOnly } from "../../hooks/useLibraryReadOnly";
import { useSidebar } from "../../hooks/useSidebar";
import { useTopics } from "../../hooks/useTopics";
import { useSites } from "../../hooks/useSites";
import { useShortcutTargets } from "../../hooks/useShortcutTargets";
import cx from "../../utils/cx";
import FeedList from "./FeedList";
import ShortcutList from "./ShortcutList";
import css from "./Sidebar.module.css";
import TopicList from "./TopicList";
import TagList from "./TagList";
import { useTags } from "../../providers/TagsContext";
import SiteList from "./SiteList";
import { SidebarListLayoutContext } from "./SidebarListLayoutContext";
import type { ListType } from "../../providers/ExpandedListContext";

type SidebarProps = {
  className?: string;
  filter: string;
  onFilterChange: (filter: string) => void;
  onAddFeed: () => void;
  onAddTopic: () => void;
  libraryBadgeRef?: (element: HTMLSpanElement | null) => void;
};

export function Sidebar({
  className,
  filter,
  onFilterChange,
  onAddFeed,
  onAddTopic,
  libraryBadgeRef,
}: SidebarProps) {
  const toolbarRef = useRef<HTMLDivElement>(null);
  const listsRef = useRef<HTMLDivElement>(null);
  const [compact, setCompact] = useState(false);
  const [actions, setActions] = useState<HTMLSpanElement | null>(null);
  const [listOptions, setListOptions] = useState<
    { value: ListType; label: string }[]
  >([]);
  const compactRef = useRef(false);
  const transferFocus = useRef(false);
  const [, abortControllerRef] = useAbortController();
  const isLibraryReadOnly = useLibraryReadOnly();

  const { tags } = useTags();
  const {
    targets: pinnedTargets,
    missing: missingPins,
    hierarchy,
  } = useShortcutTargets();
  const { topics } = useTopics(abortControllerRef);

  const { loading, loadingFeedIds, feeds, refreshAllFeeds } =
    useFeeds(abortControllerRef);
  const { selectShortcutTarget, state } = useRouter();
  const counts = useAllEntitiesCounts();
  const allEntities = useMemo(
    () => (counts ? { ...ALL_ENTITIES, ...counts } : ALL_ENTITIES),
    [counts]
  );

  const { expandedList, setExpandedList } = useSidebar(abortControllerRef);
  const { sites } = useSites(abortControllerRef);
  const normalizedFilter = filter.trim().toLocaleLowerCase();
  const matchesFilter = useCallback(
    ({ title }: { title: string }) =>
      title.toLocaleLowerCase().includes(normalizedFilter),
    [normalizedFilter]
  );
  const filteredTopics = useMemo(
    () => topics.filter(matchesFilter),
    [matchesFilter, topics]
  );
  const filteredFeeds = useMemo(
    () => feeds.filter(matchesFilter),
    [feeds, matchesFilter]
  );
  const filteredSites = useMemo(
    () => sites.filter(matchesFilter),
    [matchesFilter, sites]
  );

  const selectedList = compact ? (expandedList ?? "SHORTCUTS") : expandedList;

  useLayoutEffect(() => {
    const region = listsRef.current;
    if (!region || typeof ResizeObserver === "undefined") return;
    const headers = [
      ...region.querySelectorAll<HTMLElement>("[data-sidebar-section-header]"),
    ];
    setListOptions(
      headers.map((header) => ({
        value: header.dataset.sidebarSectionHeader!.toUpperCase() as ListType,
        label: header.dataset.sectionLabel!,
      }))
    );
    const update = () => {
      const total = headers.reduce(
        (sum, header) => sum + header.getBoundingClientRect().height,
        0
      );
      const next =
        total > 0 && region.getBoundingClientRect().height < 2 * total;
      if (compactRef.current !== next) {
        const focused = document.activeElement;
        transferFocus.current =
          headers.some((header) => header.contains(focused)) ||
          !!focused?.closest("[data-sidebar-compact-header]");
        compactRef.current = next;
        setCompact(next);
      }
    };
    const observer = new ResizeObserver(update);
    observer.observe(region);
    headers.forEach((header) => observer.observe(header));
    update();
    return () => observer.disconnect();
  }, []);

  useLayoutEffect(() => {
    if (!transferFocus.current) return;
    transferFocus.current = false;
    const target = compact
      ? listsRef.current?.querySelector<HTMLElement>(
          '[data-testid="sidebar-list-selector"] [role="combobox"]'
        )
      : listsRef.current?.querySelector<HTMLElement>(
          `[data-testid="sidebar-${(expandedList ?? "SHORTCUTS").toLowerCase()}-toggle"]`
        );
    target?.focus();
  }, [compact, expandedList]);

  const listSelector = (
    <TextField
      className={css.ListSelector}
      select
      size="small"
      aria-label="Collection list"
      data-testid="sidebar-list-selector"
      value={selectedList ?? "SHORTCUTS"}
      onChange={(event) => setExpandedList(event.target.value as ListType)}
      slotProps={{ select: { MenuProps: { transitionDuration: 0 } } }}
      variant="standard"
    >
      {listOptions.map(({ value, label }) => (
        <MenuItem key={value} value={value} className={css.ListOption}>
          {label}
        </MenuItem>
      ))}
    </TextField>
  );

  const handleExpandedList = useCallback(
    (listType: typeof expandedList) => {
      setExpandedList((prev) => (prev === listType ? null : listType));
    },
    [setExpandedList]
  );

  const [spin, setSpin] = useState(false);

  const handleRefreshAllFeeds = async () => {
    if (spin || loading) return;
    setSpin(true);
    try {
      await refreshAllFeeds();
    } finally {
      setSpin(false);
    }
  };

  return (
    <div className={cx(className, css.Sidebar)}>
      <div ref={toolbarRef} className={css.Buttons} data-library-toolbar>
        {isLibraryReadOnly && (
          <span className={css.LibraryBadgeSlot} ref={libraryBadgeRef} />
        )}
        <SidebarSortControl
          className={css.ToolbarButton}
          menuAnchor={() => toolbarRef.current}
          filter={filter}
          onFilterChange={onFilterChange}
        />
        <Tooltip
          title={
            isLibraryReadOnly
              ? "This library is read-only"
              : spin || loading
                ? "Refresh in progress…"
                : "Refresh"
          }
        >
          {/* span keeps the Tooltip working when the button is disabled */}
          <span className={css.RefreshWrap}>
            <IconButton
              data-testid="refresh"
              className={cx(css.ToolbarButton, spin && css.Loading)}
              size="small"
              aria-label="Refresh"
              disabled={isLibraryReadOnly || spin || loading}
              onClick={() => handleRefreshAllFeeds()}
            >
              <Refresh fontSize="small" />
            </IconButton>
          </span>
        </Tooltip>
      </div>

      <List dense disablePadding className={css.AllItems}>
        <SidebarEntity
          testId="all-items"
          icon={Newspaper}
          collection={allEntities}
          loading={loadingFeedIds.size > 0}
          selected={
            !state.articleScopeId && state.shortcutId === ALL_ENTITIES.id
          }
          onSelectCollection={selectShortcutTarget}
          renderSecondaryAction={
            counts
              ? () => <UnreadCount count={counts.unreadCount} />
              : undefined
          }
        />
      </List>

      <div ref={listsRef} className={css.Lists} data-testid="sidebar-lists">
        {compact && (
          <div className={css.CompactHeader} data-sidebar-compact-header>
            {listSelector}
            <span ref={setActions} className={css.CompactActions} />
          </div>
        )}

        <SidebarListLayoutContext.Provider value={{ compact, actions }}>
          <ShortcutList
            shortcuts={pinnedTargets}
            hierarchy={hierarchy}
            filter={filter}
            missing={missingPins}
            onClickHeader={() => handleExpandedList("SHORTCUTS")}
            isExpanded={selectedList === "SHORTCUTS"}
          />

          <TopicList
            topics={filteredTopics}
            addDisabled={isLibraryReadOnly}
            onAddTopic={onAddTopic}
            onClickHeader={() => handleExpandedList("TOPICS")}
            isExpanded={selectedList === "TOPICS"}
          />

          <FeedList
            feeds={filteredFeeds}
            addDisabled={isLibraryReadOnly}
            onAddFeed={onAddFeed}
            onClickHeader={() => handleExpandedList("FEEDS")}
            isExpanded={selectedList === "FEEDS"}
          />

          <SiteList
            sites={filteredSites}
            onClickHeader={() => handleExpandedList("SITES")}
            isExpanded={selectedList === "SITES"}
          />

          <TagList
            tags={tags.filter((tag) =>
              tag.name.toLowerCase().includes(normalizedFilter)
            )}
            isExpanded={selectedList === "TAGS"}
            onClickHeader={() => handleExpandedList("TAGS")}
          />
        </SidebarListLayoutContext.Provider>
      </div>
    </div>
  );
}
