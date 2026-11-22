/* oxlint-disable react/only-export-components -- shared destination control and its selection helpers */
import { TextField } from "@mui/material";
import { useCallback, useState } from "react";
import { errorMessage, isAppError, type AppError } from "../../platform/errors";
import { usePins } from "../../providers/PinsContext";
import {
  listShortcutDestinations,
  pinKey,
  type PinNode,
  type PinTarget,
} from "../../pins";

const noShortcut = "none";
export const topLevel = "root";

function shortcutLocation(nodes: PinNode[], target: PinTarget): string {
  const key = pinKey(target);
  const find = (children: PinNode[], location: string): string | undefined => {
    for (const node of children) {
      if (node.type === "group") {
        const found = find(node.children, `group:${node.id}`);
        if (found !== undefined) return found;
      } else if (pinKey(node) === key) {
        return location;
      }
    }
  };
  return find(nodes, topLevel) ?? noShortcut;
}

export function shortcutPlacement(selection: string, moveExisting: boolean) {
  return {
    parent: selection === topLevel ? null : selection.slice("group:".length),
    moveExisting,
  };
}

export function shortcutSaveError(
  entity: "Feed" | "Topic",
  error: unknown
): AppError {
  return {
    code: "app:error",
    message: `The ${entity} was saved. Its shortcut could not be saved. ${errorMessage(error)}`,
    detail: isAppError(error) ? error.detail : "",
  };
}

export function useShortcutSelection(target?: PinTarget) {
  const { tree, setPin } = usePins();
  const current = target ? shortcutLocation(tree, target) : noShortcut;
  const [edit, setEdit] = useState<{ initial: string; value: string }>();
  const reset = useCallback(() => setEdit(undefined), []);
  return {
    value: edit?.value ?? current,
    change(value: string) {
      setEdit((previous) => ({ initial: previous?.initial ?? current, value }));
    },
    reset,
    async save(savedTarget: PinTarget) {
      if (!edit || edit.value === edit.initial) return;
      if (edit.value === noShortcut) await setPin(savedTarget, false);
      else await setPin(savedTarget, true, shortcutPlacement(edit.value, true));
    },
  };
}

export default function ShortcutDestination({
  value,
  onChange,
  disabled,
  allowNone = true,
  autoFocus = false,
}: {
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
  allowNone?: boolean;
  autoFocus?: boolean;
}) {
  const { tree } = usePins();
  const groups = listShortcutDestinations(tree);
  return (
    <TextField
      select
      label="Shortcut destination"
      autoFocus={autoFocus}
      fullWidth
      value={value}
      onChange={(event) => onChange(event.target.value)}
      disabled={disabled}
      slotProps={{
        select: {
          native: true,
          inputProps: { "data-testid": "shortcut-location" },
        },
        inputLabel: { shrink: true },
      }}
    >
      {allowNone && <option value={noShortcut}>No shortcut</option>}
      <option value={topLevel}>Top level</option>
      {value.startsWith("group:") &&
        !groups.some(({ id }) => `group:${id}` === value) && (
          <option value={value} disabled>
            Folder unavailable
          </option>
        )}
      {groups.map(({ id, title }) => (
        <option key={id} value={`group:${id}`}>
          {title}
        </option>
      ))}
    </TextField>
  );
}
