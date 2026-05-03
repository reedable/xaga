import { Circle } from "@mui/icons-material";
import {
  Collapse,
  LinearProgress,
  ListItemButton,
  ListItemText,
  Typography,
} from "@mui/material";
import { useCallback, useEffect, useState } from "react";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import { Virtuoso } from "react-virtuoso";
import { useEvents } from "../../hooks/useEvents";
import useSearch from "../../hooks/useSearch";
import { isAnnotation, isEntry, type Embedded, type Item } from "../../types";
import cx from "../../utils/cx";
import formatByline from "../../utils/formatByline";
import Quotes from "../Quotes";
import css from "./_.module.css";

interface SearchResultListProps {
  className?: string;
  searchQuery: string;
}

export function SearchResultList({ className, searchQuery }: SearchResultListProps) {
  const navigate = useNavigate();
  const { collectionId, itemId: selectedItemId } = useParams();
  const [searchParams] = useSearchParams();

  const { loading, error, search } = useSearch();
  const { onUnreadChanged } = useEvents();
  const [results, setResults] = useState<(Embedded & Item)[]>([]);

  useEffect(() => {
    (async () => {
      setResults([]);
      const r = await search(searchQuery);
      if (r) setResults(r);
    })();
  }, [searchQuery, search]);

  useEffect(() => {
    return onUnreadChanged((entry) => {
      setResults((prev) =>
        prev.map((p) => (p.id === entry.id ? { ...p, ...entry } : p))
      );
    });
  }, [onUnreadChanged]);

  const handleClickItem = useCallback(
    (item: Item) => {
      const params = new URLSearchParams(searchParams);
      navigate(`/${collectionId}/${item.id}?${params.toString()}`);
    },
    [navigate, collectionId, searchParams],
  );

  return (
    <div className={cx(className, css.ItemList)}>
      <div className={css.Header}>
        <Typography variant="overline" color="text.secondary">
          Results for: {searchQuery}
        </Typography>
      </div>

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

      {!error && !loading && results.length === 0 && (
        <div className={cx(css.List, css.Placeholder)}>
          <Typography variant="body2" color="text.disabled">
            No results for "{searchQuery}"
          </Typography>
        </div>
      )}

      {results.length > 0 && (
        <Virtuoso
          className={css.List}
          data={results}
          itemContent={(_index, item) => (
            <div key={item.id} className={css.ListItem}>
              <ListItemButton
                className={css.ListItemButton}
                selected={selectedItemId === item.id}
                onClick={() => handleClickItem(item)}
              >
                <ListItemText
                  primary={
                    <Quotes quoted={isAnnotation(item)}>{item.title}</Quotes>
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
          )}
        />
      )}

      {loading && results.length === 0 && (
        <div className={cx(css.List, css.Placeholder)}>
          <Typography variant="body2" color="text.disabled">
            Searching…
          </Typography>
        </div>
      )}
    </div>
  );
}
