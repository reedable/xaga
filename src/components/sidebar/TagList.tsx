import { Checkbox, ListItemButton, ListItemText, Typography } from "@mui/material";
import type { Tag } from "../../domain";
import useRouter from "../../hooks/useRouter";
import { useTags } from "../../providers/TagsContext";
import ExpandableList from "./ExpandableList";

export default function TagList({
  tags,
  isExpanded,
  onClickHeader,
}: {
  tags: Tag[];
  isExpanded: boolean;
  onClickHeader: () => void;
}) {
  const { state, selectTags } = useRouter();
  const { tags: definitions, unreadCount, hasLoaded } = useTags();
  const selected = state.tagIds ?? [];
  const options = [
    ...tags,
    ...selected
      .filter((id) => !definitions.some((tag) => tag.id === id))
      .map((id) => ({ id, name: "Unavailable tag" })),
  ];

  return (
    <ExpandableList
      label="Tags"
      count={tags.length}
      isExpanded={isExpanded}
      onClickHeader={onClickHeader}
    >
      {!hasLoaded && <Typography role="status">Loading tags…</Typography>}

      {options.map((tag) => (
        <ListItemButton
          key={tag.id}
          role="checkbox"
          aria-checked={selected.includes(tag.id)}
          selected={selected.includes(tag.id)}
          onClick={() =>
            selectTags(
              selected.includes(tag.id)
                ? selected.filter((id) => id !== tag.id)
                : [...selected, tag.id],
            )
          }
        >
          <Checkbox
            checked={selected.includes(tag.id)}
            tabIndex={-1}
            disableRipple
            slotProps={{ input: { "aria-hidden": true } }}
          />
          <ListItemText primary={tag.name} />
        </ListItemButton>
      ))}
    </ExpandableList>
  );
}
