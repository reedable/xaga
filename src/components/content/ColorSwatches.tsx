import { Close } from "@mui/icons-material";
import { Button, IconButton, Tooltip } from "@mui/material";
import { useState } from "react";
import { COLOR_NAMES } from "../../colors";
import colorCss from "../../colors.module.css";
import cx from "../../utils/cx";
import css from "./AnnotationTooltip.module.css";

/** Shared palette for passage highlights and whole-article markings. */
export function ColorSwatches({
  color,
  onChange,
  disabled = false,
  autoFocus = false,
}: {
  color: string | null;
  onChange: (color: string | null) => void;
  disabled?: boolean;
  autoFocus?: boolean;
}) {
  const [pendingRemove, setPendingRemove] = useState(false);
  return (
    <div className={css.ColorRow}>
      <Tooltip
        title="Remove marking"
        slotProps={{ transition: { timeout: 0 } }}
      >
        <span>
          <IconButton
            disableRipple
            sx={{ transition: "none" }}
            size="small"
            aria-label="Remove marking"
            disabled={disabled || color === null}
            onClick={() => {
              if (pendingRemove) {
                onChange(null);
                setPendingRemove(false);
              } else setPendingRemove(true);
            }}
          >
            <Close fontSize="small" />
          </IconButton>
        </span>
      </Tooltip>
      {pendingRemove && (
        <Button
          disableRipple
          sx={{ transition: "none" }}
          disabled={disabled}
          onClick={() => {
            onChange(null);
            setPendingRemove(false);
          }}
        >
          Remove
        </Button>
      )}
      {COLOR_NAMES.map((name, index) => (
        <Tooltip
          key={name}
          title={name}
          slotProps={{ transition: { timeout: 0 } }}
        >
          <span>
            <IconButton
              disableRipple
              sx={{ transition: "none" }}
              size="small"
              aria-label={name}
              aria-pressed={color === name}
              autoFocus={
                autoFocus && (color === name || (color === null && index === 0))
              }
              disabled={disabled}
              onClick={() => {
                setPendingRemove(false);
                onChange(name);
              }}
            >
              <span
                className={cx(
                  css.Swatch,
                  colorCss[name],
                  name === color && css.Selected
                )}
              />
            </IconButton>
          </span>
        </Tooltip>
      ))}
    </div>
  );
}
