import { pinKey } from "../../pins";
import { type ShortcutNode } from "../../utils/projectPinTree";
import { Clear, Refresh, Settings } from "@mui/icons-material";
import {
  Checkbox,
  IconButton,
  InputAdornment,
  ListSubheader,
  MenuItem,
  Select,
  TextField,
  ToggleButton,
  ToggleButtonGroup,
  Tooltip,
  Typography,
  type SelectChangeEvent,
} from "@mui/material";
import { type MouseEvent } from "react";
import { SHORTCUTS } from "../../Shortcuts";
import { isEntry, isTopic, type TopicSort } from "../../types";
import cx from "../../utils/cx";
import MissingShortcutsButton from "../sidebar/MissingShortcutsButton";
import { TagActions } from "../TagActions";
import css from "./EntityList.module.css";

import type { useEntityListData } from "./useEntityListData";

type HeaderProps = {
  data: ReturnType<typeof useEntityListData>;
  headerRef: React.Ref<HTMLDivElement>;
  readOnly: boolean;
  isSelectableHeader: boolean;
  handleEdit: (() => void) | undefined;
  headerOpen: boolean;
  setHeaderOpen: (open: boolean) => void;
  headerValues: string[];
  handleCollectionChange: (event: SelectChangeEvent<string[]>) => void;
  handleFilter: (event: MouseEvent<HTMLElement>, value: string) => void;
};

export function EntityListHeader({
  data,
  headerRef,
  readOnly,
  isSelectableHeader,
  handleEdit,
  headerOpen,
  setHeaderOpen,
  headerValues,
  handleCollectionChange,
  handleFilter,
}: HeaderProps) {
  const {
    selectedScope,
    tagIds,
    refreshSpin,
    loadingFeeds,
    handleRefresh,
    shortcutHierarchy,
    articleScopeId,
    articleOptions,
    shortcutTargets,
    topics,
    feeds,
    tagOptions,
    deleting,
    merging,
    renaming,
    tags,
    siteOptions,
    missingPins,
    filter,
    hasSelectedTags,
    topicSort,
    setTopicSort,
    filterText,
    setFilterText,
  } = data;
  function shortcutOptions(
    nodes: ShortcutNode[],
    depth = 0
  ): React.ReactNode[] {
    return nodes.flatMap((node): React.ReactNode[] => {
      const key = pinKey(node.reference);
      if (!node.children) {
        return [
          <MenuItem
            key={`pin:${key}`}
            value={`pin:${key}`}
            disabled={!node.target}
            sx={{ pl: 2 + depth * 2 }}
          >
            {node.title}
          </MenuItem>,
        ];
      }
      const children = node.children.length
        ? shortcutOptions(node.children, depth + 1)
        : [
            <MenuItem key={`empty:${key}`} disabled sx={{ pl: 4 + depth * 2 }}>
              Empty
            </MenuItem>,
          ];
      return [
        <ListSubheader key={key} sx={{ pl: 2 + depth * 2 }}>
          {node.title}
        </ListSubheader>,
        ...children,
      ];
    });
  }

  return (
    <div
      ref={headerRef}
      className={css.Header}
      tabIndex={-1}
      data-testid="entity-list-header"
    >
      <div className={cx(css.HeadingWrapper)}>
        {!isSelectableHeader && (
          <>
            <Typography
              data-testid="all-items"
              className={css.Heading}
              component="div"
              color="text.secondary"
              variant="overline"
              sx={{ userSelect: "none" }}
            >
              {selectedScope?.title ?? "All items"}
            </Typography>

            {handleEdit && (
              <Tooltip title="Edit">
                <IconButton
                  data-testid="edit-collection"
                  size="small"
                  onClick={handleEdit}
                >
                  <Settings sx={{ fontSize: 16 }} />
                </IconButton>
              </Tooltip>
            )}
          </>
        )}

        {isSelectableHeader && (
          <div className={css.HeadingControl}>
            <Tooltip
              title={
                readOnly && tagIds === undefined
                  ? "This library is read-only"
                  : "Refresh"
              }
            >
              <span className={css.RefreshControl}>
                <IconButton
                  className={cx(
                    css.RefreshButton,
                    refreshSpin && css.RefreshLoading
                  )}
                  size="small"
                  data-testid="refresh-collection"
                  aria-label="Refresh collection"
                  disabled={
                    tagIds === undefined &&
                    (readOnly || refreshSpin || loadingFeeds)
                  }
                  onClick={handleRefresh}
                >
                  <Refresh fontSize="small" />
                </IconButton>
              </span>
            </Tooltip>

            <Select
              className={css.CollectionSelect}
              size="small"
              MenuProps={{
                onKeyDownCapture: (event) => {
                  if (
                    event.key !== "Tab" ||
                    !event.currentTarget.contains(event.target as Node)
                  )
                    return;
                  const buttons = [
                    ...event.currentTarget.querySelectorAll<HTMLButtonElement>(
                      "[data-tag-action]"
                    ),
                  ];
                  const target = event.target as HTMLElement;
                  const current = buttons.indexOf(target as HTMLButtonElement);
                  const rowButton = target
                    .closest(".TagActionRow")
                    ?.querySelector<HTMLButtonElement>("[data-tag-action]");
                  const next =
                    current >= 0
                      ? buttons[current + (event.shiftKey ? -1 : 1)]
                      : event.shiftKey
                        ? undefined
                        : rowButton;
                  if (next) {
                    event.preventDefault();
                    event.stopPropagation();
                    next.focus();
                  }
                },
              }}
              multiple
              open={headerOpen}
              onOpen={() => setHeaderOpen(true)}
              onClose={() => setHeaderOpen(false)}
              value={headerValues}
              renderValue={() => selectedScope.title}
              SelectDisplayProps={{
                role: "combobox",
                ...{ "data-testid": "collection-selector" },
              }}
              inputProps={{
                "aria-label": "Browse collections and tags",
              }}
              onChange={handleCollectionChange}
            >
              <ListSubheader>Shortcuts</ListSubheader>
              {SHORTCUTS.map((s) => (
                <MenuItem key={s.id} value={s.id}>
                  <Typography component="span" variant="body2">
                    {s.title}
                  </Typography>
                </MenuItem>
              ))}
              {shortcutOptions(shortcutHierarchy)}
              {articleScopeId &&
                !articleOptions.some(
                  (entry) => entry.id === articleScopeId
                ) && (
                  <MenuItem value={`article:${articleScopeId}`} disabled>
                    Loading article…
                  </MenuItem>
                )}
              {articleOptions
                .filter(
                  (entry) =>
                    !shortcutTargets.some(
                      (target) => isEntry(target) && target.id === entry.id
                    )
                )
                .map((entry) => (
                  <MenuItem
                    key={`article:${entry.id}`}
                    value={`article:${entry.id}`}
                  >
                    <Typography component="span" variant="body2">
                      {entry.title}
                    </Typography>
                  </MenuItem>
                ))}
              {topics.length > 0 && <ListSubheader>Topics</ListSubheader>}
              {topics.map((t) => (
                <MenuItem key={t.id} value={t.id}>
                  <Typography component="span" variant="body2">
                    {t.title}
                  </Typography>
                </MenuItem>
              ))}
              {feeds.length > 0 && <ListSubheader>Feeds</ListSubheader>}
              {feeds.map((f) => (
                <MenuItem key={f.id} value={f.id}>
                  <Typography component="span" variant="body2">
                    {f.title}
                  </Typography>
                </MenuItem>
              ))}
              <ListSubheader>Tags</ListSubheader>
              <MenuItem value="tags:all">All items (Tags)</MenuItem>
              {tagOptions.map((tag) => (
                <MenuItem
                  className="TagActionRow"
                  key={`tag:${tag.id}`}
                  value={`tag:${tag.id}`}
                >
                  <Checkbox
                    checked={tagIds?.includes(tag.id) ?? false}
                    tabIndex={-1}
                    slotProps={{ input: { "aria-hidden": true } }}
                  />
                  <Typography
                    component="span"
                    variant="body2"
                    sx={{ flexGrow: 1 }}
                  >
                    {tag.name}
                    {deleting.includes(tag.id)
                      ? " — Deleting…"
                      : merging.some((merge) => merge.survivorId === tag.id)
                        ? " — Merging…"
                        : renaming.includes(tag.id)
                          ? " — Renaming…"
                          : ""}
                  </Typography>
                  {tags.some((entity) => entity.id === tag.id) && (
                    <TagActions tag={tag} />
                  )}
                </MenuItem>
              ))}
              {siteOptions.length > 0 && <ListSubheader>Sites</ListSubheader>}
              {siteOptions.map((site) => (
                <MenuItem key={site.id} value={site.id}>
                  <Typography component="span" variant="body2">
                    {site.title}
                  </Typography>
                </MenuItem>
              ))}
            </Select>
            <MissingShortcutsButton targets={missingPins} />
          </div>
        )}
      </div>

      {!articleScopeId && (
        <ToggleButtonGroup
          data-testid="filter"
          aria-label="Filter"
          className={css.FilterButtonGroup}
          exclusive
          value={filter}
          onChange={handleFilter}
        >
          {!hasSelectedTags && (
            <ToggleButton
              data-testid="filter-unread"
              size="small"
              value="unread"
              aria-label="Unread"
            >
              Unread
            </ToggleButton>
          )}
          <ToggleButton
            data-testid="filter-saved"
            size="small"
            value="saved"
            aria-label="Saved"
          >
            Saved
          </ToggleButton>
          <ToggleButton
            data-testid="filter-all"
            size="small"
            value="all"
            aria-label="All"
          >
            All
          </ToggleButton>
        </ToggleButtonGroup>
      )}

      {isTopic(selectedScope) && (
        <Select
          className={css.SortSelect}
          size="small"
          value={topicSort}
          variant="standard"
          onChange={(e) => setTopicSort(e.target.value as TopicSort)}
        >
          <MenuItem value="relevance">Sort by most relevant</MenuItem>
          <MenuItem value="timestamp">Sort by most recent</MenuItem>
        </Select>
      )}

      <TextField
        data-testid="filter-items"
        className={css.FilterInput}
        size="small"
        fullWidth
        placeholder="Filter items…"
        value={filterText}
        onChange={(e) => setFilterText(e.target.value)}
        variant="standard"
        slotProps={{
          input: {
            endAdornment: filterText ? (
              <InputAdornment position="end">
                <Tooltip title="Clear Filter">
                  <IconButton
                    size="small"
                    aria-label="Clear filter"
                    onClick={() => setFilterText("")}
                    edge="end"
                  >
                    <Clear fontSize="small" />
                  </IconButton>
                </Tooltip>
              </InputAdornment>
            ) : null,
          },
        }}
        sx={{
          "& .MuiInputBase-input": {
            paddingLeft: "0 !important",
          },
        }}
      />
    </div>
  );
}
