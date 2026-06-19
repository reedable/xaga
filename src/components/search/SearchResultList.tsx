import { Circle } from "@mui/icons-material";
import {
  Collapse,
  LinearProgress,
  List,
  ListItemButton,
  ListItemText,
  ToggleButton,
  ToggleButtonGroup,
  Typography,
} from "@mui/material";
import { useEffect, useMemo, useState } from "react";
import useAbortController from "../../hooks/useAbortController";
import { useEvents } from "../../hooks/useEvents";
import { useFeeds } from "../../hooks/useFeeds";
import useRouter from "../../hooks/useRouter";
import useSearch from "../../hooks/useSearch";
import {
  isAnnotation,
  isEntry,
  isFeed,
  isTopic,
  type Ranked,
  type Item,
} from "../../types";
import cx from "../../utils/cx";
import formatByline from "../../utils/formatByline";
import Quotes from "../Quotes";
import { SearchInput } from "../SearchInput";
import css from "./SearchResultList.module.css";

type TypeFilter = "entries" | "topics" | "annotations" | "feeds";

interface SearchResultListProps {
  className?: string;
  searchQuery: string;
  onSaveAsTopic?: (title: string) => void;
}

export function SearchResultList({
  className,
  searchQuery,
  onSaveAsTopic,
}: SearchResultListProps) {
  const { selectItem, state, clearSearch } = useRouter();
  const { entryId } = state;
  const { loading, error, search } = useSearch();
  const { onUnreadChanged, onBookmarkChanged } = useEvents();
  const [, abortControllerRef] = useAbortController();
  const { feeds } = useFeeds(abortControllerRef);
  const [results, setResults] = useState<Ranked<Item>[]>([]);
  const [typeFilter, setTypeFilter] = useState<TypeFilter>("entries");

  const visibleResults = useMemo((): Item[] => {
    if (typeFilter === "topics") return results.filter(isTopic);
    if (typeFilter === "annotations") return results.filter(isAnnotation);
    if (typeFilter === "feeds") {
      // Hybrid: name-matches first (local title/URL filter over all feeds), then the
      // semantic feed hits from the unified search appended after, deduped by id.
      const query = searchQuery.trim().toLowerCase();
      const nameMatched = query
        ? feeds.filter(
            (feed) =>
              feed.title.toLowerCase().includes(query) ||
              feed.url.toLowerCase().includes(query)
          )
        : [];
      const seen = new Set(nameMatched.map((feed) => feed.id));
      const semantic = results.filter(isFeed).filter((feed) => !seen.has(feed.id));
      return [...nameMatched, ...semantic];
    }
    return results.filter(isEntry);
  }, [results, typeFilter, feeds, searchQuery]);

  useEffect(() => {
    (async () => {
      setResults([]);

      if (searchQuery.trim().length) {
        const r = await search(searchQuery);
        if (r) setResults(r);
      }
    })();
  }, [searchQuery, search]);

  useEffect(() => {
    return onUnreadChanged((entry) => {
      setResults((prev) =>
        prev.map((p) => (p.id === entry.id ? { ...p, ...entry } : p))
      );
    });
  }, [onUnreadChanged]);

  useEffect(() => {
    return onBookmarkChanged((entry) => {
      setResults((prev) =>
        prev.map((p) => (p.id === entry.id ? { ...p, ...entry } : p))
      );
    });
  }, [onBookmarkChanged]);

  return (
    <div className={cx(className, css.ItemList)}>
      <SearchInput
        className={css.SearchInput}
        onClear={clearSearch}
        onSaveAsTopic={onSaveAsTopic}
      />

      {searchQuery && (
        <div className={css.Header}>
          <Typography variant="overline" color="text.secondary">
            Results for: {searchQuery}
          </Typography>
          <ToggleButtonGroup
            aria-label="Filter by type"
            className={css.FilterButtonGroup}
            exclusive
            value={typeFilter}
            onChange={(_, v) => {
              if (v) setTypeFilter(v);
            }}
          >
            <ToggleButton size="small" value="entries" aria-label="Entries">
              Entries
            </ToggleButton>
            <ToggleButton size="small" value="topics" aria-label="Topics">
              Topics
            </ToggleButton>
            <ToggleButton
              size="small"
              value="annotations"
              aria-label="Annotations"
            >
              Annotations
            </ToggleButton>
            <ToggleButton size="small" value="feeds" aria-label="Feeds">
              Feeds
            </ToggleButton>
          </ToggleButtonGroup>
        </div>
      )}

      <div>
        <Collapse in={loading}>
          <LinearProgress variant="indeterminate" />
        </Collapse>
      </div>

      {!!error && !loading && (
        <div className={cx(css.List, css.Placeholder)}>
          <Typography variant="body2" color="error">
            Search failed
          </Typography>
        </div>
      )}

      {!searchQuery && !loading && (
        <div className={cx(css.List, css.Placeholder)}>
          <Typography variant="body2" color="text.disabled">
            Type to search
          </Typography>
        </div>
      )}

      {searchQuery && !error && !loading && visibleResults.length === 0 && (
        <div className={cx(css.List, css.Placeholder)}>
          <Typography variant="body2" color="text.disabled">
            {`No ${typeFilter} for "${searchQuery}"`}
          </Typography>
        </div>
      )}

      {visibleResults.length > 0 && (
        <List className={css.List} disablePadding>
          {visibleResults.map((item) => (
            <div key={item.id} className={css.ListItem}>
              <ListItemButton
                className={css.ListItemButton}
                selected={entryId === item.id}
                onClick={() => selectItem(item)}
              >
                <ListItemText
                  primary={
                    <Quotes
                      quoted={isAnnotation(item)}
                      color={isAnnotation(item) ? item.color : undefined}
                    >
                      {item.title}
                    </Quotes>
                  }
                  secondary={
                    <>
                      {isEntry(item) && item.isUnread && (
                        <Circle className={css.UnreadIcon} color="primary" />
                      )}
                      <span>{formatByline(item)}</span>
                    </>
                  }
                  slotProps={{
                    primary: { className: css.Primary },
                    secondary: { className: css.Secondary },
                  }}
                />
              </ListItemButton>
            </div>
          ))}
        </List>
      )}

      {searchQuery && loading && results.length === 0 && (
        <div className={cx(css.List, css.Placeholder)}>
          <Typography variant="body2" color="text.disabled">
            Searching…
          </Typography>
        </div>
      )}
    </div>
  );
}
