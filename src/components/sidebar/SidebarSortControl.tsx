import { ListSubheader, MenuItem, Select, Typography } from "@mui/material";
import type { SidebarSortMode } from "../../hooks/useSidebarSort";
import css from "./SidebarSortControl.module.css";

type SidebarSortControlProps = {
  label: string;
  sortMode: SidebarSortMode;
  onChange: (sortMode: SidebarSortMode) => void;
};

export default function SidebarSortControl({ label, sortMode, onChange }: SidebarSortControlProps) {
  return (
    <ListSubheader className={css.SortControl} component="li" disableSticky>
      <Select
        aria-label={`Sort ${label}`}
        className={css.SortSelect}
        fullWidth
        size="small"
        value={sortMode}
        variant="standard"
        onChange={(event) => onChange(event.target.value as SidebarSortMode)}
        sx={{ height: 24 }}
      >
        <MenuItem value="unread">Sort by unread count</MenuItem>
        <MenuItem value="alphabetical">Sort alphabetically</MenuItem>
      </Select>
    </ListSubheader>
  );
}
