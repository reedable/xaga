import { ArrowDownward, ArrowUpward, Close } from "@mui/icons-material";
import {
  IconButton,
  InputBase,
  Paper,
  Tooltip,
  Typography,
} from "@mui/material";
import { useEffect, useRef } from "react";
import cx from "../../utils/cx";
import css from "./FindBar.module.css";

interface FindBarProps {
  query: string;
  current: number;
  total: number;
  onChange: (query: string) => void;
  onNavigate: (direction: 1 | -1) => void;
  onClose: () => void;
}

export function FindBar({
  query,
  current,
  total,
  onChange,
  onNavigate,
  onClose,
}: FindBarProps) {
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Escape") {
      onClose();
    } else if (e.key === "Enter") {
      e.preventDefault();
      onNavigate(e.shiftKey ? -1 : 1);
    }
  }

  const noResults = query.length > 0 && total === 0;

  return (
    <Paper elevation={4} className={css.FindBar}>
      <InputBase
        inputRef={inputRef}
        value={query}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="Find in article…"
        inputProps={{ "aria-label": "Find in article" }}
        className={cx(css.Input, noResults && css.InputNoResults)}
      />

      <Typography
        variant="caption"
        color={noResults ? "error" : "text.secondary"}
        className={css.Count}
      >
        {noResults ? "No results" : total > 0 ? `${current} / ${total}` : ""}
      </Typography>

      <Tooltip title="Previous match">
        <IconButton
          size="small"
          onClick={() => onNavigate(-1)}
          disabled={total === 0}
          aria-label="Previous match"
        >
          <ArrowUpward fontSize="inherit" />
        </IconButton>
      </Tooltip>
      <Tooltip title="Next match">
        <IconButton
          size="small"
          onClick={() => onNavigate(1)}
          disabled={total === 0}
          aria-label="Next match"
        >
          <ArrowDownward fontSize="inherit" />
        </IconButton>
      </Tooltip>
      <Tooltip title="Close find bar">
        <IconButton size="small" onClick={onClose} aria-label="Close find bar">
          <Close fontSize="inherit" />
        </IconButton>
      </Tooltip>
    </Paper>
  );
}
