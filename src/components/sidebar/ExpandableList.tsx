import { ChevronRight } from "@mui/icons-material";
import { Collapse, List, ToggleButton, Typography } from "@mui/material";
import { type ReactNode } from "react";
import cx from "../../utils/cx";
import css from "./ExpandableList.module.css";

export interface ExpandableListProps {
  children?: ReactNode;
  className?: string;
  count: number;
  label: string;
  section?: string;
  isExpanded: boolean;
  onClickHeader: () => void;
  renderHeaderAction?: () => ReactNode;
}

export default function ExpandableList({
  children,
  className,
  count,
  label,
  section,
  isExpanded,
  onClickHeader,
  renderHeaderAction,
}: ExpandableListProps) {
  return (
    <>
      <div className={cx(className, css.ExpandableListHeaderWrapper)}>
        <ToggleButton
          data-testid={section && `sidebar-${section}-toggle`}
          className={css.ExpandableListHeader}
          selected={isExpanded}
          value="checked"
          onClick={onClickHeader}
          disableTouchRipple
          focusRipple
        >
          <ChevronRight className={css.ChevronIcon} />

          <Typography className={css.Label} component="div" variant="overline">
            {label} ({count})
          </Typography>
        </ToggleButton>

        {renderHeaderAction && (
          <span
            onClick={(e) => e.stopPropagation()}
            className={css.ExpandableListHeaderAction}
          >
            {renderHeaderAction()}
          </span>
        )}
      </div>

      <div
        // Keep outgoing rows out of input and accessibility queries while Collapse animates.
        inert={!isExpanded}
        aria-hidden={!isExpanded}
        className={cx(
          className,
          css.ExpanderContent,
          isExpanded && css.Expanded
        )}
      >
        <Collapse in={isExpanded}>
          <List
            data-testid={section && `sidebar-${section}-list`}
            aria-label={label}
            dense
            disablePadding
          >
            {children}
          </List>
        </Collapse>
      </div>
    </>
  );
}
