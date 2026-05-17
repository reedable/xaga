import { Circle } from "@mui/icons-material";
import {
  Collapse,
  LinearProgress,
  List,
  ListItemButton,
  ListItemText,
  Typography,
} from "@mui/material";
import { useEffect, useState } from "react";
import { useEvents } from "../../hooks/useEvents";
import useRouter from "../../hooks/useRouter";
import useSearch from "../../hooks/useSearch";
import { isAnnotation, isEntry, type Embedded, type Item } from "../../types";
import cx from "../../utils/cx";
import formatByline from "../../utils/formatByline";
import Quotes from "../Quotes";
import css from "./SearchResultList.module.css";
import { SearchInput } from "../SearchInput";

interface SearchResultListProps {
  className?: string;
  searchQuery: string;
}

export function SearchResultList({
  className,
  searchQuery,
}: SearchResultListProps) {
  const { show, state, clearSearch } = useRouter();
  const { entryId } = state;
  const { loading, error, search } = useSearch();
  const { onUnreadChanged, onBookmarkChanged } = useEvents();
  const [results, setResults] = useState<(Embedded & Item)[]>([]);

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
      <SearchInput className={css.SearchInput} onClear={clearSearch} />

      {searchQuery && (
        <div className={css.Header}>
          <Typography variant="overline" color="text.secondary">
            Results for: {searchQuery}
          </Typography>
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

      {searchQuery && !error && !loading && results.length === 0 && (
        <div className={cx(css.List, css.Placeholder)}>
          <Typography variant="body2" color="text.disabled">
            No results for "{searchQuery}"
          </Typography>
        </div>
      )}

      {results.length > 0 && (
        <List className={css.List} disablePadding>
          {results.map((item) => (
            <div key={item.id} className={css.ListItem}>
              <ListItemButton
                className={css.ListItemButton}
                selected={entryId === item.id}
                onClick={() => show(item)}
              >
                <ListItemText
                  primary={
                    <Quotes quoted={isAnnotation(item)} color={isAnnotation(item) ? item.color : undefined}>{item.title}</Quotes>
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
