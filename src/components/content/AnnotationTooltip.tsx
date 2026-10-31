import { InfoOutlined } from "@mui/icons-material";
import {
  ClickAwayListener,
  Paper,
  TextField,
  IconButton,
  Tooltip,
} from "@mui/material";
import { useCallback, useEffect, useState } from "react";
import colorCss from "../../colors.module.css";
import cx from "../../utils/cx";
import { ColorSwatches } from "./ColorSwatches";
import css from "./AnnotationTooltip.module.css";

interface AnnotationTooltipProps {
  text: string[];
  note?: string | null;
  color: string | null;
  position: { x: number; y: number };
  onChange: (color: string | null, note: string) => void;
  onClose: () => void;
  onOpenInspector: (opener: HTMLButtonElement) => Promise<void>;
  inspectorOpen: boolean;
}

export function AnnotationTooltip({
  color,
  note,
  position,
  onChange,
  onClose,
  onOpenInspector,
  inspectorOpen,
}: AnnotationTooltipProps) {
  const initialNote = note ?? "";
  const [noteValue, setNoteValue] = useState(initialNote);

  const handleNoteKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Escape") setNoteValue(initialNote);
    },
    [initialNote]
  );

  useEffect(() => {
    window.addEventListener("blur", onClose);
    return () => window.removeEventListener("blur", onClose);
  }, [onClose]);

  const colorClass = color
    ? colorCss[color as keyof typeof colorCss]
    : undefined;

  return (
    <ClickAwayListener
      onClickAway={(event) => {
        // The drawer's close click can reach this listener after its state
        // update. Preserve where that click originated, even after unmount.
        if (
          event
            .composedPath()
            .some(
              (node) =>
                node instanceof Element &&
                node.hasAttribute("data-inspector-drawer")
            )
        )
          return;
        onClose();
      }}
    >
      <Paper
        elevation={4}
        className={cx(css.Tooltip, colorClass, inspectorOpen && css.Inspecting)}
        inert={inspectorOpen}
        style={{ top: position.y, left: position.x }}
      >
        <div className={css.Body}>
          {color !== null && (
            <TextField
              multiline
              minRows={1}
              maxRows={6}
              size="small"
              variant="standard"
              value={noteValue}
              onChange={(e) => {
                setNoteValue(e.target.value);
                onChange(color, e.target.value);
              }}
              placeholder="Add a note…"
              onKeyDown={handleNoteKeyDown}
              className={css.NoteField}
            />
          )}

          <div className={css.ColorRow}>
            <ColorSwatches
              color={color}
              onChange={(next) => onChange(next, noteValue)}
            />

            {color !== null && (
              <div className={css.ColorRowAction}>
                <Tooltip
                  title="Open Inspector"
                  slotProps={{ transition: { timeout: 0 } }}
                >
                  <IconButton
                    size="small"
                    aria-label="Open Inspector"
                    onClick={(event) =>
                      void onOpenInspector(event.currentTarget)
                    }
                  >
                    <InfoOutlined fontSize="small" />
                  </IconButton>
                </Tooltip>
              </div>
            )}
          </div>
        </div>
      </Paper>
    </ClickAwayListener>
  );
}
