import { useEntityListData } from "./useEntityListData";
import { EntityListHeader } from "./EntityListHeader";
import { EntityListRow } from "./EntityListRow";
import { useReadingPane } from "../../providers/ReadingPaneContext";
import { pinKey } from "../../pins";
import { shortcutReference } from "../../utils/projectPinTree";
import { useLibraryReadOnly } from "../../hooks/useLibraryReadOnly";
import {
  Button,
  Collapse,
  LinearProgress,
  Tooltip,
  Typography,
  type SelectChangeEvent,
} from "@mui/material";
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type MouseEvent,
} from "react";
import { Virtuoso, type VirtuosoHandle } from "react-virtuoso";
import { useLocation, useNavigationType } from "react-router-dom";
import useRouter from "../../hooks/useRouter";
import { SHORTCUTS } from "../../Shortcuts";
import {
  isFilter,
  isFeed,
  isManagedCollection,
  isSite,
  isTopic,
  type Feed,
  type Filter,
  type Site,
  type Topic,
} from "../../types";
import cx from "../../utils/cx";
import css from "./EntityList.module.css";

function registerReturnFocus(
  ref: React.RefObject<(() => void) | null>,
  restore: () => void
) {
  ref.current = restore;
  return () => {
    if (ref.current === restore) ref.current = null;
  };
}

function getEmptyMessage(filter: Filter, filterText: string): string {
  const label = filter === "all" ? "items" : `${filter} items`;
  return filterText ? `No ${label} match "${filterText}"` : `No ${label}`;
}

type EntityListProps = {
  filter: Filter;
  onFilterChange: (filter: Filter) => void;
  className?: string;
  isSelectableHeader: boolean;
  onEditFeed: (feed: Feed) => void;
  onEditTopic: (topic: Topic) => void;
  onEditSite?: (site: Site) => void;
};

export function EntityList({
  filter: listFilter,
  onFilterChange,
  className,
  isSelectableHeader,
  onEditFeed,
  onEditTopic,
  onEditSite,
}: EntityListProps) {
  const readingPane = useReadingPane();
  const headerRef = useRef<HTMLDivElement>(null);
  const readOnly = useLibraryReadOnly();
  const location = useLocation();
  const navigationType = useNavigationType();
  const lastLocationKey = useRef(location.key);
  const pendingHistorySelection = useRef<string | undefined>(undefined);
  const {
    selectEntity,
    selectCollection,
    selectShortcut,
    selectShortcutTarget,
    selectArticleScope,
    replaceAnnotationSelection,
    state,
    selectTags,
  } = useRouter();
  const [headerOpen, setHeaderOpen] = useState(false);
  const selectedEntityId = state.annotationId ?? state.entryId;
  const data = useEntityListData({
    state,
    listFilter,
    readOnly,
    replaceAnnotationSelection,
  });
  const {
    tagIds,
    filter,
    pinnedEntries,
    shortcutTargets,
    articleScopeId,
    feedId,
    topicId,
    siteHostname,
    topics,
    feeds,
    loadingFeeds,
    loading,
    selectedScope,
    siteOptions,
    entities,
    filterText,
    displayEntities,
    listLoading,
    handleMarkEntriesRead,
  } = data;

  // Virtuoso only renders on-screen rows, so moving the selection off-screen
  // requires an imperative scroll to bring the target row into view before it
  // can receive DOM focus. itemRefs holds the focusable node of each rendered
  // row, keyed by entity id.
  const virtuosoRef = useRef<VirtuosoHandle>(null);
  const itemRefs = useRef(new Map<string, HTMLDivElement>());

  const handleEdit = useMemo(() => {
    if (isSite(selectedScope)) return () => onEditSite?.(selectedScope);
    if (!isManagedCollection(selectedScope)) return undefined;
    if (isFeed(selectedScope)) return () => onEditFeed(selectedScope);
    if (isTopic(selectedScope)) return () => onEditTopic(selectedScope);
    return undefined;
  }, [selectedScope, onEditFeed, onEditTopic, onEditSite]);

  const activePinKey = articleScopeId
    ? `pin:entry:${articleScopeId}`
    : feedId
      ? `pin:feed:${feedId}`
      : topicId
        ? `pin:topic:${topicId}`
        : siteHostname
          ? `pin:site:${siteHostname}`
          : undefined;
  const scopeValues = [
    articleScopeId ? `article:${articleScopeId}` : selectedScope.id,
  ];
  if (
    activePinKey &&
    shortcutTargets.some(
      (target) => `pin:${pinKey(shortcutReference(target))}` === activePinKey
    )
  ) {
    scopeValues.push(activePinKey);
  }
  const headerValues =
    tagIds !== undefined
      ? tagIds.length
        ? tagIds.map((id) => `tag:${id}`)
        : ["tags:all"]
      : scopeValues;
  const handleCollectionChange = (event: SelectChangeEvent<string[]>) => {
    const values =
      typeof event.target.value === "string"
        ? event.target.value.split(",")
        : event.target.value;
    const id =
      values.find((value) => !headerValues.includes(value)) ??
      headerValues.find((value) => !values.includes(value));
    if (!id) return;
    if (id === "tags:all") {
      selectTags([]);
      return;
    }
    if (id.startsWith("tag:")) {
      selectTags(
        values
          .filter((value) => value.startsWith("tag:"))
          .map((value) => value.slice(4))
      );
      return;
    }
    setHeaderOpen(false);
    if (id.startsWith("pin:")) {
      const target = shortcutTargets.find(
        (target) => `pin:${pinKey(shortcutReference(target))}` === id
      );
      if (target) selectShortcutTarget(target);
      return;
    }
    const article = pinnedEntries.find((entry) => `article:${entry.id}` === id);
    if (article) {
      selectArticleScope(article);
      return;
    }
    const shortcut = SHORTCUTS.find((entity) => entity.id === id);
    if (shortcut) {
      selectShortcut(shortcut);
      return;
    }
    const collection = [...topics, ...feeds, ...siteOptions].find(
      (entity) => entity.id === id
    );
    if (collection) selectCollection(collection);
  };

  const returnFocus = readingPane?.returnFocus;
  useEffect(() => {
    if (!returnFocus) return;
    const restore = () => {
      const index = displayEntities.findIndex(
        (entity) => entity.id === state.entryId
      );
      if (index < 0) {
        headerRef.current?.focus({ preventScroll: true });
        return;
      }
      const focus = () =>
        (
          itemRefs.current.get(displayEntities[index].id) ?? headerRef.current
        )?.focus({ preventScroll: true });
      if (itemRefs.current.has(displayEntities[index].id)) focus();
      else
        virtuosoRef.current?.scrollIntoView({
          index,
          behavior: "auto",
          done: focus,
        });
    };
    return registerReturnFocus(returnFocus, restore);
  }, [returnFocus, displayEntities, state.entryId]);

  // Up/Down move the selection to the adjacent visible entity, clamped at both
  // ends (no wrap). Selecting reuses selectEntity, so it loads the entity's content
  // and pushes history exactly like a click; the newly selected row is then
  // scrolled into view and given DOM focus so keyboard operation can continue
  // from there (WCAG 2.1.1).
  const handleEntityKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
      if (e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return;

      const currentIndex = displayEntities.findIndex(
        (i) => itemRefs.current.get(i.id) === e.currentTarget
      );
      if (currentIndex === -1) return;

      const nextIndex = currentIndex + (e.key === "ArrowDown" ? 1 : -1);
      if (nextIndex < 0 || nextIndex >= displayEntities.length) return;

      e.preventDefault();
      const next = displayEntities[nextIndex];
      selectEntity(next);
      virtuosoRef.current?.scrollIntoView({
        index: nextIndex,
        behavior: "auto",
        done: () => itemRefs.current.get(next.id)?.focus(),
      });
    },
    [displayEntities, selectEntity]
  );

  const handleFilter = useCallback(
    (_event: MouseEvent<HTMLElement>, value: string) => {
      if (isFilter(value)) onFilterChange(value);
    },
    [onFilterChange]
  );

  useEffect(() => {
    if (lastLocationKey.current !== location.key) {
      lastLocationKey.current = location.key;
      if (navigationType === "POP")
        pendingHistorySelection.current = selectedEntityId;
      // Reader URL/presentation replacements may arrive while the list loads.
      else if (
        navigationType === "PUSH" ||
        pendingHistorySelection.current !== selectedEntityId
      )
        pendingHistorySelection.current = undefined;
    }
    const id = pendingHistorySelection.current;
    if (!id || listLoading.current) return;
    pendingHistorySelection.current = undefined;
    const index = displayEntities.findIndex((entity) => entity.id === id);
    if (index !== -1)
      virtuosoRef.current?.scrollIntoView({ index, behavior: "auto" });
  }, [
    listLoading,
    location.key,
    navigationType,
    selectedEntityId,
    displayEntities,
    loading,
  ]);

  return (
    <div className={cx(className, css.EntityList)}>
      <EntityListHeader
        data={data}
        headerRef={headerRef}
        readOnly={readOnly}
        isSelectableHeader={isSelectableHeader}
        handleEdit={handleEdit}
        headerOpen={headerOpen}
        setHeaderOpen={setHeaderOpen}
        headerValues={headerValues}
        handleCollectionChange={handleCollectionChange}
        handleFilter={handleFilter}
      />

      <div>
        <Collapse in={loading}>
          <LinearProgress
            variant="indeterminate"
            color={loadingFeeds ? "warning" : "primary"}
          />
        </Collapse>
      </div>

      {!loading && entities.length === 0 && (
        <div className={cx(css.List, css.Placeholder)}>
          <Typography component="div" variant="body2" color="text.disabled">
            {getEmptyMessage(articleScopeId ? "all" : (filter ?? "unread"), "")}
          </Typography>
        </div>
      )}

      {entities.length > 0 && displayEntities.length === 0 && (
        <div className={cx(css.List, css.Placeholder)}>
          <Typography component="div" variant="body2" color="text.disabled">
            {getEmptyMessage(
              articleScopeId ? "all" : (filter ?? "unread"),
              filterText
            )}
          </Typography>
        </div>
      )}

      {displayEntities.length > 0 && (
        <Virtuoso
          data-testid="entity-list-scroller"
          ref={virtuosoRef}
          className={css.List}
          data={displayEntities}
          itemContent={(_index, entity) => (
            <EntityListRow
              entity={entity}
              selected={selectedEntityId === entity.id}
              rowRef={(node) => {
                if (node) itemRefs.current.set(entity.id, node);
                else itemRefs.current.delete(entity.id);
              }}
              tabIndex={
                selectedEntityId === entity.id ||
                (!displayEntities.some(
                  (value) => value.id === selectedEntityId
                ) &&
                  displayEntities[0].id === entity.id)
                  ? 0
                  : -1
              }
              onKeyDown={handleEntityKeyDown}
              onClick={() => selectEntity(entity)}
            />
          )}
        />
      )}

      {loading && entities.length === 0 && (
        <div className={cx(css.List, css.Placeholder)}>
          <Typography component="div" variant="body2" color="text.disabled">
            Loading...
          </Typography>
        </div>
      )}

      {!articleScopeId && (
        <div className={css.Toolbar}>
          <Tooltip
            title={readOnly ? "This library is read-only" : "Mark all read"}
          >
            <span>
              <Button
                data-testid="mark-all-read"
                disabled={readOnly}
                onClick={handleMarkEntriesRead}
              >
                Mark all read
              </Button>
            </span>
          </Tooltip>
        </div>
      )}
    </div>
  );
}
