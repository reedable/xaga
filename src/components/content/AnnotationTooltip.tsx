import { Close } from "@mui/icons-material";
import {
  Button,
  ClickAwayListener,
  Collapse,
  Paper,
  TextField,
  Tooltip,
} from "@mui/material";
import { forwardRef, useCallback, useEffect, useImperativeHandle, useRef, useState } from "react";
import colorCss from "../../colors.module.css";
import { COLOR_NAMES } from "../../colors";
import type { Annotation, Entry } from "../../types";
import cx from "../../utils/cx";
import css from "./AnnotationTooltip.module.css";
import { TopicMenuButton, type TopicMenuButtonHandle } from "./TopicMenuButton";

export interface AnnotationTooltipHandle {
  saveAnnotationToTopics: (annotation: Entry | Annotation) => Promise<void>;
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

function SwatchTarget({
  onClick,
  className,
  children,
}: {
  onClick: () => void;
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <div
      className={`${css.SwatchTarget}${className ? ` ${className}` : ""}`}
      onClick={onClick}
    >
      {children}
    </div>
  );
}

export const AnnotationTooltip = forwardRef<AnnotationTooltipHandle, AnnotationTooltipProps>(
  function AnnotationTooltip({
    annotation,
    color,
    note,
    position,
    onChange,
    onClose,
  }, ref) {
    const collectionMenuRef = useRef<TopicMenuButtonHandle | null>(null);

    const initialNote = note ?? "";
    const [noteValue, setNoteValue] = useState(initialNote);
    const [pendingRemove, setPendingRemove] = useState(false);

    useImperativeHandle(ref, () => ({
      saveAnnotationToTopics: (item: Entry | Annotation) =>
        collectionMenuRef.current?.saveToTopics(item) ?? Promise.resolve(),
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

    const colorClass = color ? colorCss[color as keyof typeof colorCss] : undefined;

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

            <div className={cx(css.ColorRow, pendingRemove && css.RemovePending)}>
              <SwatchTarget
                className={css.Remove}
                onClick={() => {
                  if (pendingRemove) {
                    onChange(null, noteValue);
                    setPendingRemove(false);
                  } else {
                    setPendingRemove(true);
                  }
                }}
              >
                <Tooltip title="Remove Annotation">
                  <>
                    <Close className="swatch-inner" />
                    <Collapse in={pendingRemove} orientation="horizontal">
                      <Button>Remove</Button>
                    </Collapse>
                  </>
                </Tooltip>
              </SwatchTarget>

              {COLOR_NAMES.map((name) => (
                <SwatchTarget
                  key={name}
                  onClick={() => {
                    setPendingRemove(false);
                    onChange(name, noteValue);
                  }}
                >
                  <Tooltip title={name}>
                    <div
                      className={cx(
                        "swatch-inner",
                        css.Swatch,
                        colorCss[name as keyof typeof colorCss],
                        name === color && css.Selected,
                      )}
                    />
                  </Tooltip>
                </SwatchTarget>
              ))}

              <div className={css.ColorRowAction}>
                <TopicMenuButton ref={collectionMenuRef} item={annotation ?? null} />
              </div>
            </div>
          </div>
        </Paper>
      </ClickAwayListener>
    );
  }
);
