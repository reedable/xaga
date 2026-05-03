import { Clear, Search } from "@mui/icons-material";
import {
  CircularProgress,
  IconButton,
  InputAdornment,
  TextField,
} from "@mui/material";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useRef, useState } from "react";
import { useSearchParams } from "react-router-dom";
import css from "./_.module.css";

interface TitleBarProps {
  className?: string;
}

const DEBOUNCE_MS = 300;

export function TitleBar({ className }: TitleBarProps) {
  const [, setSearchParams] = useSearchParams();
  const [inputValue, setInputValue] = useState("");
  const [isExpanded, setIsExpanded] = useState(false);
  const [isPending, setIsPending] = useState(false);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const updateSearch = useCallback(
    (value: string) =>
      setSearchParams((prev) => {
        if (value) prev.set("search", value);
        else prev.delete("search");
        return prev;
      }),
    [setSearchParams],
  );

  const commitSearch = useCallback(
    (value: string) => {
      setIsPending(false);
      updateSearch(value);
    },
    [updateSearch],
  );

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const value = e.target.value;
      setInputValue(value);

      if (debounceRef.current) clearTimeout(debounceRef.current);

      if (!value) {
        setIsPending(false);
        updateSearch("");
        return;
      }

      setIsPending(true);
      debounceRef.current = setTimeout(() => commitSearch(value), DEBOUNCE_MS);
    },
    [commitSearch, updateSearch],
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
    updateSearch("");
  }, [updateSearch]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Escape") handleClear();
    },
    [handleClear],
  );

  const handleBlur = useCallback(() => {
    if (!inputValue) setIsExpanded(false);
  }, [inputValue]);

  const handleMouseDown = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    const target = e.target as HTMLElement;
    if (target.closest("button, input, a, textarea")) return;
    void getCurrentWindow().startDragging();
  }, []);

  const handleDoubleClick = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      const target = e.target as HTMLElement;
      if (target.closest("button, input, a, textarea")) return;
      void getCurrentWindow().toggleMaximize();
    },
    [],
  );

  const classes = [css.TitleBar, className].filter(Boolean).join(" ");

  return (
    <div
      className={classes}
      onMouseDown={handleMouseDown}
      onDoubleClick={handleDoubleClick}
    >
      <div className={css.TrafficLightSpacer} />

      <div className={css.SearchArea}>
        {!isExpanded ? (
          <IconButton size="small" onClick={handleExpand} aria-label="Search">
            <Search fontSize="small" />
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
                      <Search fontSize="small" />
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
    </div>
  );
}
