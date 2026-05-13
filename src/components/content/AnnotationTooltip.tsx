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
import { useAppearance } from "../../hooks/useAppearance";
import {
  ANNOTATION_COLORS_DARK,
  ANNOTATION_COLORS_LIGHT,
  getAnnotationColor,
} from "../../text/annotationColors";
import type { Annotation, Entry } from "../../types";
import cx from "../../utils/cx";
import css from "./AnnotationTooltip.module.css";
import { CollectionMenuButton, type CollectionMenuButtonHandle } from "./CollectionMenuButton";

export interface AnnotationTooltipHandle {
  saveAnnotationToCollections: (annotation: Entry | Annotation) => Promise<void>;
}

export interface AnnotationTooltipProps {
  annotation?: Annotation | null;
  text: string[];
  note?: string | null;
  color: string;
  position: { x: number; y: number };
  onChange: (color: string, note: string) => void;
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
    const collectionMenuRef = useRef<CollectionMenuButtonHandle | null>(null);

    const { themeMode } = useAppearance();
    const dark = themeMode === "dark";
    const annotationColors = dark
      ? ANNOTATION_COLORS_DARK
      : ANNOTATION_COLORS_LIGHT;

    const initialNote = note ?? "";
    const [noteValue, setNoteValue] = useState(initialNote);
    const [pendingRemove, setPendingRemove] = useState(false);

    useImperativeHandle(ref, () => ({
      saveAnnotationToCollections: (item: Entry | Annotation) =>
        collectionMenuRef.current?.saveToCollections(item) ?? Promise.resolve(),
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

    return (
      <ClickAwayListener onClickAway={onClose}>
        <Paper
          elevation={4}
          className={css.Tooltip}
          style={{
            top: position.y,
            left: position.x,
            borderLeft: `4px solid ${getAnnotationColor(color, dark).accent}`,
          }}
        >
          <div className={css.Body}>
            <Collapse in={color !== "none"}>
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
              {Object.entries(annotationColors).map(([name, tokens]) =>
                name === "none" ? (
                  <SwatchTarget
                    key={name}
                    className={css.Remove}
                    onClick={() => {
                      if (pendingRemove) {
                        onChange("none", noteValue);
                        setPendingRemove(false);
                      } else {
                        setPendingRemove(true);
                      }
                    }}
                  >
                    <Tooltip title="Remove Annotation">
                      <>
                        <Close className={`swatch-inner ${css.Swatch}`} />

                        <Collapse in={pendingRemove} orientation="horizontal">
                          <Button>Remove</Button>
                        </Collapse>
                      </>
                    </Tooltip>
                  </SwatchTarget>
                ) : (
                  <SwatchTarget
                    key={name}
                    onClick={() => {
                      setPendingRemove(false);
                      onChange(name, noteValue);
                    }}
                  >
                    <Tooltip title={name}>
                      <div
                        className={`swatch-inner ${css.Swatch}`}
                        style={{
                          background: tokens.accent,
                          outline:
                            name === color
                              ? `2px solid ${tokens.accent}`
                              : "none",
                          outlineOffset: "2px",
                        }}
                      />
                    </Tooltip>
                  </SwatchTarget>
                )
              )}

              <div className={css.ColorRowAction}>
                <CollectionMenuButton ref={collectionMenuRef} item={annotation ?? null} />
              </div>
            </div>
          </div>
        </Paper>
      </ClickAwayListener>
    );
  }
);
