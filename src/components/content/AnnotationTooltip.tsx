import { ClickAwayListener, Collapse, Paper, TextField } from "@mui/material";
import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useRef,
  useState,
} from "react";
import colorCss from "../../colors.module.css";
import type { Annotation, Entry } from "../../types";
import cx from "../../utils/cx";
import { ColorSwatches } from "./ColorSwatches";
import css from "./AnnotationTooltip.module.css";
import { TopicMenuButton, type TopicMenuButtonHandle } from "./TopicMenuButton";

export interface AnnotationTooltipHandle {
  addAnnotationToTopics: (annotation: Entry | Annotation) => Promise<void>;
}

export interface AnnotationTooltipProps {
  annotation?: Annotation | null;
  text: string[];
  note?: string | null;
  color: string | null;
  position: { x: number; y: number };
  onChange: (color: string | null, note: string) => void;
  onClose: () => void;
}

export const AnnotationTooltip = forwardRef<
  AnnotationTooltipHandle,
  AnnotationTooltipProps
>(function AnnotationTooltip(
  { annotation, color, note, position, onChange, onClose },
  ref
) {
  const collectionMenuRef = useRef<TopicMenuButtonHandle | null>(null);

  const initialNote = note ?? "";
  const [noteValue, setNoteValue] = useState(initialNote);

  useImperativeHandle(ref, () => ({
    addAnnotationToTopics: (item: Entry | Annotation) =>
      collectionMenuRef.current?.addToTopics(item) ?? Promise.resolve(),
  }));

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
    <ClickAwayListener onClickAway={onClose}>
      <Paper
        elevation={4}
        className={cx(css.Tooltip, colorClass)}
        style={{ top: position.y, left: position.x }}
      >
        <div className={css.Body}>
          <Collapse in={color !== null}>
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
          </Collapse>

          <div className={css.ColorRow}>
            <ColorSwatches
              color={color}
              onChange={(next) => onChange(next, noteValue)}
            />

            <Collapse in={color !== null} orientation="horizontal">
              <div className={css.ColorRowAction}>
                <TopicMenuButton
                  ref={collectionMenuRef}
                  item={annotation ?? null}
                />
              </div>
            </Collapse>
          </div>
        </div>
      </Paper>
    </ClickAwayListener>
  );
});
