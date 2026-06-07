import { Clear, Search as SearchIcon } from "@mui/icons-material";
import {
  CircularProgress,
  IconButton,
  InputAdornment,
  TextField,
  Tooltip,
} from "@mui/material";
import { useCallback, useRef, useState } from "react";
import useRouter from "../hooks/useRouter";
import cx from "../utils/cx";
import css from "./SearchInput.module.css";

const DEBOUNCE_MS = 300;

export type SearchInputProps = {
  className?: string;
  onClear?: () => void;
};

export function SearchInput({ className, onClear }: SearchInputProps) {
  const { invokeSearch, search } = useRouter();
  const [inputValue, setInputValue] = useState(search ?? "");
  const [isPending, setIsPending] = useState(false);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const commitSearch = useCallback(
    (value: string) => {
      setIsPending(false);
      invokeSearch(value);
    },
    [invokeSearch]
  );

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const value = e.target.value;
      setInputValue(value);
      if (debounceRef.current) clearTimeout(debounceRef.current);
      if (!value) {
        setIsPending(false);
        invokeSearch("");
        return;
      }
      setIsPending(true);
      debounceRef.current = setTimeout(() => commitSearch(value), DEBOUNCE_MS);
    },
    [commitSearch, invokeSearch]
  );

  const handleClear = useCallback(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    setInputValue("");
    setIsPending(false);
    invokeSearch("");
    onClear?.();
  }, [invokeSearch, onClear]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Escape") handleClear();
    },
    [handleClear]
  );

  return (
    <div className={cx(className, css.SearchInput)}>
      <TextField
        autoFocus
        className={css.TextField}
        size="small"
        placeholder="Search…"
        value={inputValue}
        onChange={handleChange}
        onKeyDown={handleKeyDown}
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
            endAdornment: (
              <InputAdornment position="end">
                <Tooltip title="Clear">
                  <IconButton
                    size="small"
                    onClick={handleClear}
                    edge="end"
                    aria-label="Clear"
                  >
                    <Clear fontSize="small" />
                  </IconButton>
                </Tooltip>
              </InputAdornment>
            ),
          },
        }}
      />
    </div>
  );
}
