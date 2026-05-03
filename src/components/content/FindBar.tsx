import { ArrowDownward, ArrowUpward, Close } from "@mui/icons-material";
import { IconButton, InputBase, Paper, Typography } from "@mui/material";
import { useEffect, useRef } from "react";

interface FindBarProps {
  query: string;
  current: number;
  total: number;
  onChange: (query: string) => void;
  onNavigate: (direction: 1 | -1) => void;
  onClose: () => void;
}

export function FindBar({ query, current, total, onChange, onNavigate, onClose }: FindBarProps) {
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
    <Paper
      elevation={4}
      sx={{
        alignItems: "center",
        borderRadius: 1,
        display: "flex",
        gap: 0.5,
        position: "absolute",
        px: 1,
        py: 0.5,
        right: 8,
        top: 8,
        zIndex: 100,
      }}
    >
      <InputBase
        inputRef={inputRef}
        value={query}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="Find in article…"
        inputProps={{ "aria-label": "Find in article" }}
        sx={{
          fontSize: "0.875rem",
          minWidth: 160,
          "& input": { color: noResults ? "error.main" : undefined },
        }}
      />

      <Typography
        variant="caption"
        color={noResults ? "error" : "text.secondary"}
        sx={{ minWidth: 44, textAlign: "center" }}
      >
        {noResults ? "No results" : total > 0 ? `${current} / ${total}` : ""}
      </Typography>

      <IconButton size="small" onClick={() => onNavigate(-1)} disabled={total === 0} aria-label="Previous match">
        <ArrowUpward fontSize="inherit" />
      </IconButton>
      <IconButton size="small" onClick={() => onNavigate(1)} disabled={total === 0} aria-label="Next match">
        <ArrowDownward fontSize="inherit" />
      </IconButton>
      <IconButton size="small" onClick={onClose} aria-label="Close find bar">
        <Close fontSize="inherit" />
      </IconButton>
    </Paper>
  );
}
