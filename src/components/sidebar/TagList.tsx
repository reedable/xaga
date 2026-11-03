import { Checkbox, ListItemButton, ListItemText, Typography } from "@mui/material";
import type { Tag } from "../../domain";
import useRouter from "../../hooks/useRouter";
import { useTags } from "../../providers/TagsContext";
import ExpandableList from "./ExpandableList";
import { TagActions } from "../TagActions";
import css from "./TagList.module.css";

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
  const { tags: definitions, hasLoaded, renaming = [], deleting = [] } = useTags();
  const selected = state.tagIds ?? [];

  const options = [
    ...tags,
    ...selected
      .filter((id) => !definitions.some((tag) => tag.id === id))
      .map((id) => ({ id, name: "Unavailable tag" })),
  ].sort((a, b) => a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: "base" }));

  return (
    <ExpandableList
      className={css.TagList}
      label="Tags"
      count={tags.length}
      isExpanded={isExpanded}
      onClickHeader={onClickHeader}
    >
      {!hasLoaded && <Typography role="status">Loading tags…</Typography>}

      {options.map((tag) => (
        <div key={tag.id} className="TagActionRow" style={{ display: "flex", alignItems: "center" }}>
        <ListItemButton
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
          <ListItemText primary={tag.name} secondary={deleting.includes(tag.id) ? "Deleting…" : renaming.includes(tag.id) ? "Renaming…" : undefined} />
        </ListItemButton>
        {definitions.some((item) => item.id === tag.id) && <TagActions tag={tag} />}
        </div>
      ))}
    </ExpandableList>
  );
}
