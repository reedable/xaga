import { Close } from "@mui/icons-material";
import {
  Box,
  ClickAwayListener,
  Collapse,
  Paper,
  TextField,
  Tooltip,
} from "@mui/material";
import { useCallback, useEffect, useState } from "react";
import { useAppearance } from "../../hooks/useAppearance";
import {
  ANNOTATION_COLORS_DARK,
  ANNOTATION_COLORS_LIGHT,
  getAnnotationColor,
} from "../../text/annotationColors";

export interface AnnotationTooltipProps {
  text: string[];
  note?: string | null;
  color: string;
  position: { x: number; y: number };
  onChange: (color: string, note: string) => void;
  onClose: () => void;
}

function SwatchTarget({
  onClick,
  title,
  children,
}: {
  onClick: () => void;
  title?: string;
  children: React.ReactNode;
}) {
  return (
    <Box
      onClick={onClick}
      title={title}
      sx={{
        width: 24,
        height: 24,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        cursor: "pointer",
        flexShrink: 0,
        "& > .swatch-inner": { transition: "transform 0.1s" },
        "&:hover > .swatch-inner": { transform: "scale(1.25)" },
      }}
    >
      {children}
    </Box>
  );
}

export function AnnotationTooltip({
  color,
  note,
  position,
  onChange,
  onClose,
}: AnnotationTooltipProps) {
  const { themeMode } = useAppearance();
  const dark = themeMode === "dark";
  const annotationColors = dark ? ANNOTATION_COLORS_DARK : ANNOTATION_COLORS_LIGHT;

  const initialNote = note ?? "";
  const [noteValue, setNoteValue] = useState(initialNote);

  const handleNoteKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Escape") setNoteValue(initialNote);
    },
    [initialNote],
  );

  useEffect(() => {
    window.addEventListener("blur", onClose);
    return () => window.removeEventListener("blur", onClose);
  }, [onClose]);

  return (
    <ClickAwayListener onClickAway={onClose}>
      <Paper
        elevation={4}
        sx={{
          position: "fixed",
          top: position.y,
          left: position.x,
          zIndex: 9999,
          minWidth: 320,
          maxWidth: 320,
          borderLeft: `4px solid ${getAnnotationColor(color, dark).accent}`,
        }}
      >
        <Box sx={{ display: "flex", flexDirection: "column", p: 1, gap: 0.5 }}>
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
              sx={{ width: "100%" }}
            />
          </Collapse>

          <Box sx={{ display: "flex", alignItems: "center" }}>
            {Object.entries(annotationColors).map(([name, tokens]) => (
              <SwatchTarget
                key={name}
                onClick={() => onChange(name, noteValue)}
                title={name}
              >
                {name === "none" ? (
                  <Tooltip title="none">
                    <Close
                      className="swatch-inner"
                      sx={{
                        width: 16,
                        height: 16,
                        borderRadius: "50%",
                      }}
                    />
                  </Tooltip>
                ) : (
                  <Tooltip title={name}>
                    <Box
                      className="swatch-inner"
                      sx={{
                        width: 16,
                        height: 16,
                        borderRadius: "50%",
                        background: tokens.accent,
                        outline:
                          name === color
                            ? `2px solid ${tokens.accent}`
                            : "none",
                        outlineOffset: "2px",
                      }}
                    />
                  </Tooltip>
                )}
              </SwatchTarget>
            ))}
          </Box>
        </Box>
      </Paper>
    </ClickAwayListener>
  );
}
