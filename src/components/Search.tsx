import { Clear, Search as SearchIcon } from "@mui/icons-material";
import {
  CircularProgress,
  IconButton,
  InputAdornment,
  TextField,
} from "@mui/material";
import { useCallback, useRef, useState } from "react";
import useRouter from "../hooks/useRouter";
import css from "./Search.module.css";

const DEBOUNCE_MS = 300;

export function Search() {
  const { search, searchParams } = useRouter();
  const [inputValue, setInputValue] = useState(searchParams.get("search"));
  const [isExpanded, setIsExpanded] = useState(false);
  const [isPending, setIsPending] = useState(false);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const commitSearch = useCallback(
    (value: string) => {
      setIsPending(false);
      search(value);
    },
    [search]
  );

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const value = e.target.value;
      setInputValue(value);
      if (debounceRef.current) clearTimeout(debounceRef.current);
      if (!value) {
        setIsPending(false);
        search("");
        return;
      }
      setIsPending(true);
      debounceRef.current = setTimeout(() => commitSearch(value), DEBOUNCE_MS);
    },
    [commitSearch, search]
  );

  const handleExpand = useCallback(() => {
    setIsExpanded(true);
    setTimeout(() => inputRef.current?.focus(), 0);
  }, []);

  const handleClear = useCallback(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    setInputValue("");
    setIsPending(false);
    setIsExpanded(false);
    search("");
  }, [search]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Escape") handleClear();
    },
    [handleClear]
  );

  const handleBlur = useCallback(() => {
    if (!inputValue) setIsExpanded(false);
  }, [inputValue]);

  return (
    <div className={css.Search}>
      {!isExpanded ? (
        <IconButton size="small" onClick={handleExpand} aria-label="Search">
          <SearchIcon fontSize="small" />
        </IconButton>
      ) : (
        <TextField
          inputRef={inputRef}
          size="small"
          placeholder="Search…"
          value={inputValue}
          onChange={handleChange}
          onKeyDown={handleKeyDown}
          onBlur={handleBlur}
          autoComplete="off"
          slotProps={{
            input: {
              autoComplete: "off",
              autoCorrect: "off",
              autoCapitalize: "off",
              spellCheck: false,
              startAdornment: (
                <InputAdornment position="start">
                  {isPending ? (
                    <CircularProgress size={14} />
                  ) : (
                    <SearchIcon fontSize="small" />
                  )}
                </InputAdornment>
              ),
              endAdornment: inputValue ? (
                <InputAdornment position="end">
                  <IconButton size="small" onClick={handleClear} edge="end">
                    <Clear fontSize="small" />
                  </IconButton>
                </InputAdornment>
              ) : undefined,
            },
          }}
        />
      )}
    </div>
  );
}
