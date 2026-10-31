import { Delete } from "@mui/icons-material";
import {
  Button,
  ButtonBase,
  Collapse,
  IconButton,
  Tooltip,
  useMediaQuery,
} from "@mui/material";
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
  const animate = useMediaQuery("(prefers-reduced-motion: no-preference)");
  const remove = () => {
    onChange(null);
    setPendingRemove(false);
  };
  return (
    <div className={cx(css.ColorRow, pendingRemove && css.RemovePending)}>
      <div className={cx(css.SwatchTarget, css.Remove)}>
        <Tooltip
          title="Remove marking"
          slotProps={{ transition: { timeout: 0 } }}
        >
          <span>
            <IconButton
              disableRipple
              className={css.RemoveButton}
              aria-label="Remove marking"
              disabled={disabled || color === null}
              onClick={() =>
                pendingRemove ? remove() : setPendingRemove(true)
              }
            >
              <Delete />
            </IconButton>
          </span>
        </Tooltip>
        <Collapse
          in={pendingRemove}
          orientation="horizontal"
          timeout={animate ? undefined : 0}
        >
          <Button
            disableRipple
            sx={{ transition: "none" }}
            disabled={disabled || !pendingRemove}
            tabIndex={pendingRemove ? 0 : -1}
            onClick={remove}
          >
            Remove
          </Button>
        </Collapse>
      </div>
      {COLOR_NAMES.map((name, index) => (
        <div key={name} className={css.SwatchTarget}>
          <Tooltip title={name} slotProps={{ transition: { timeout: 0 } }}>
            <ButtonBase
              disableRipple
              className={css.SwatchButton}
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
            </ButtonBase>
          </Tooltip>
        </div>
      ))}
    </div>
  );
}
