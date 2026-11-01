import { Clear, Search as SearchIcon } from "@mui/icons-material";
import {
  CircularProgress,
  IconButton,
  InputAdornment,
  Menu,
  MenuItem,
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
  onSaveAsTopic?: (title: string) => void;
  disabled?: boolean;
};

export function SearchInput({
  className,
  onClear,
  onSaveAsTopic,
  disabled = false,
}: SearchInputProps) {
  const { invokeSearch, search } = useRouter();
  const [inputValue, setInputValue] = useState(search ?? "");
  const [isPending, setIsPending] = useState(false);
  const [menuAnchor, setMenuAnchor] = useState<HTMLElement | null>(null);
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

  const handleMenuOpen = useCallback(
    (e: React.MouseEvent<HTMLButtonElement>) => {
      setMenuAnchor(e.currentTarget);
    },
    []
  );

  const handleMenuClose = useCallback(() => {
    setMenuAnchor(null);
  }, []);

  const handleSaveAsTopic = useCallback(() => {
    setMenuAnchor(null);
    onSaveAsTopic?.(inputValue.trim());
  }, [inputValue, onSaveAsTopic]);

  return (
    <div className={cx(className, css.SearchInput)}>
      <TextField
        autoFocus
        className={css.TextField}
        size="small"
        placeholder="Search…"
        value={inputValue}
        disabled={disabled}
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
                  <Tooltip title="Search menu">
                    <IconButton
                      size="small"
                      edge="start"
                      onClick={handleMenuOpen}
                      aria-label="Search menu"
                    >
                      <SearchIcon fontSize="small" />
                    </IconButton>
                  </Tooltip>
                )}
              </InputAdornment>
            ),
            endAdornment: (
              <InputAdornment position="end">
                <Tooltip title="Clear Search">
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

      <Menu
        anchorEl={menuAnchor}
        open={menuAnchor !== null}
        onClose={handleMenuClose}
      >
        <MenuItem disabled={!inputValue.trim()} onClick={handleSaveAsTopic}>
          Save as Topic
        </MenuItem>
      </Menu>
    </div>
  );
}
