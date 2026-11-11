import { Clear, FilterList } from "@mui/icons-material";
import {
  Badge,
  IconButton,
  InputAdornment,
  MenuItem,
  Popover,
  TextField,
  Tooltip,
  Typography,
} from "@mui/material";
import { useId, useRef, useState } from "react";
import { useSidebarSort } from "../../hooks/useSidebarSort";
import type { SidebarSortMode } from "../../sidebarSort";

export default function SidebarSortControl({
  className,
  menuAnchor,
  filter,
  onFilterChange,
}: {
  className?: string;
  menuAnchor: () => HTMLElement | null;
  filter: string;
  onFilterChange: (filter: string) => void;
}) {
  const [sortMode, setSortMode] = useSidebarSort();
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);
  const filterRef = useRef<HTMLInputElement>(null);
  const popupId = useId();
  const titleId = useId();
  const active = !!filter.trim();

  const label = active
    ? "Sort and filter collections (filter active)"
    : "Sort and filter collections";

  return (
    <>
      <Tooltip title={label}>
        <IconButton
          data-testid="sidebar-sort-trigger"
          className={className}
          size="small"
          aria-label={label}
          aria-haspopup="dialog"
          aria-controls={anchor ? popupId : undefined}
          aria-expanded={!!anchor}
          onClick={(event) => setAnchor(event.currentTarget)}
        >
          <Badge color="primary" variant="dot" invisible={!active}>
            <FilterList fontSize="small" />
          </Badge>
        </IconButton>
      </Tooltip>

      <Popover
        transitionDuration={0}
        anchorEl={() => menuAnchor() ?? anchor!}
        anchorOrigin={{ vertical: "bottom", horizontal: "right" }}
        transformOrigin={{ vertical: "top", horizontal: "left" }}
        slotProps={{
          paper: {
            id: popupId,
            role: "dialog",
            "aria-modal": true,
            "aria-labelledby": titleId,
            sx: {
              marginLeft: "0.5rem",
              padding: "1rem",
              width: "20rem",
              maxWidth: "calc(100% - 2rem)",
              maxHeight: "calc(100% - 2rem)",
              display: "flex",
              flexDirection: "column",
              gap: "1rem",
            },
          },
        }}
        open={!!anchor}
        onClose={() => setAnchor(null)}
      >
        <Typography id={titleId} component="h2" variant="subtitle1">
          Sort and filter collections
        </Typography>

        <TextField
          select
          data-testid="sidebar-sort-select"
          autoFocus
          label="Sort collections"
          size="small"
          value={sortMode}
          slotProps={{ select: { MenuProps: { transitionDuration: 0 } } }}
          onChange={(event) =>
            void setSortMode(event.target.value as SidebarSortMode)
          }
        >
          <MenuItem value="unread">Sort by unread</MenuItem>
          <MenuItem value="alphabetical">Sort by name</MenuItem>
        </TextField>

        <TextField
          inputRef={filterRef}
          label="Filter collections"
          size="small"
          value={filter}
          onChange={(event) => onFilterChange(event.target.value)}
          slotProps={{
            htmlInput: { "data-testid": "filter-collections" },
            input: {
              endAdornment: filter ? (
                <InputAdornment position="end">
                  <Tooltip title="Clear collection filter">
                    <IconButton
                      data-testid="clear-collection-filter"
                      aria-label="Clear collection filter"
                      edge="end"
                      size="small"
                      onClick={() => {
                        onFilterChange("");
                        filterRef.current?.focus();
                      }}
                    >
                      <Clear fontSize="small" />
                    </IconButton>
                  </Tooltip>
                </InputAdornment>
              ) : null,
            },
          }}
        />
      </Popover>
    </>
  );
}
