import { Sort } from "@mui/icons-material";
import { IconButton, Menu, MenuItem, Tooltip } from "@mui/material";
import { useState } from "react";
import { useSidebarSort } from "../../hooks/useSidebarSort";

export default function SidebarSortControl({
  className,
  menuAnchor,
}: {
  className?: string;
  menuAnchor: () => HTMLElement | null;
}) {
  const [sortMode, setSortMode] = useSidebarSort();
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);
  return (
    <>
      <Tooltip title="Sort collections">
        <IconButton
          className={className}
          size="small"
          aria-label="Sort collections"
          aria-haspopup="menu"
          aria-controls={anchor ? "sidebar-sort-menu" : undefined}
          aria-expanded={!!anchor}
          onClick={(event) => setAnchor(event.currentTarget)}
        >
          <Sort fontSize="small" />
        </IconButton>
      </Tooltip>
      <Menu
        id="sidebar-sort-menu"
        anchorEl={() => menuAnchor() ?? anchor!}
        anchorOrigin={{ vertical: "bottom", horizontal: "right" }}
        transformOrigin={{ vertical: "top", horizontal: "left" }}
        slotProps={{ paper: { sx: { marginLeft: "0.5rem" } } }}
        open={!!anchor}
        onClose={() => setAnchor(null)}
      >
        {(
          [
            ["unread", "Sort by unread"],
            ["alphabetical", "Sort by name"],
          ] as const
        ).map(([value, label]) => (
          <MenuItem
            key={value}
            role="menuitemradio"
            aria-checked={sortMode === value}
            selected={sortMode === value}
            onClick={() => {
              setAnchor(null);
              void setSortMode(value);
            }}
          >
            {label}
          </MenuItem>
        ))}
      </Menu>
    </>
  );
}
