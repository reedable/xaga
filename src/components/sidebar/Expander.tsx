import { ChevronRight } from "@mui/icons-material";
import { Collapse, List, ToggleButton, Typography } from "@mui/material";
import { useCallback, type ReactNode } from "react";
import cx from "../../utils/cx";
import css from "./_.module.css";

export interface ExpanderProps {
  children?: ReactNode;
  label: string;
  isExpanded: boolean;
  onClickHeader: () => void;
  renderHeaderAction?: () => ReactNode;
}

export default function Expander({
  children,
  label,
  isExpanded,
  onClickHeader,
  renderHeaderAction,
}: ExpanderProps) {
  const handleClickHeader = useCallback(() => {
    onClickHeader();
  }, [onClickHeader]);

  return (
    <>
      <div className={css.ExpanderHeaderWrapper}>
        <ToggleButton
          className={css.ExpanderHeader}
          selected={isExpanded}
          value="checked"
          onClick={handleClickHeader}
        >
          <ChevronRight className={css.ChevronIcon} />

          <Typography
            className={css.Label}
            component="div"
            variant="overline"
            sx={{ flex: 1 }}
          >
            {label}
          </Typography>
        </ToggleButton>

        {renderHeaderAction && (
          <span onClick={(e) => e.stopPropagation()} className={css.ExpanderHeaderAction}>
            {renderHeaderAction()}
          </span>
        )}
      </div>

      <div className={cx(css.ExpanderContent, isExpanded && css.Expanded)}>
        <Collapse in={isExpanded}>
          <List dense disablePadding>
            {children}
          </List>
        </Collapse>
      </div>
    </>
  );
}
