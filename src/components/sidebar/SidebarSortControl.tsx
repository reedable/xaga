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
        fullWidth
        size="small"
        value={sortMode}
        variant="standard"
        onChange={(event) => onChange(event.target.value as SidebarSortMode)}
      >
        <MenuItem value="unread">
          <Typography component="span" variant="body2">
            Unread count
          </Typography>
        </MenuItem>
        <MenuItem value="alphabetical">
          <Typography component="span" variant="body2">
            Alphabetical
          </Typography>
        </MenuItem>
      </Select>
    </ListSubheader>
  );
}
