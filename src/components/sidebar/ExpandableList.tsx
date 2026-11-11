import { ChevronRight } from "@mui/icons-material";
import { Collapse, List, ToggleButton, Typography } from "@mui/material";
import { useContext, type ReactNode } from "react";
import { createPortal } from "react-dom";
import cx from "../../utils/cx";
import css from "./ExpandableList.module.css";
import { SidebarListLayoutContext } from "./SidebarListLayoutContext";

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
  const layout = useContext(SidebarListLayoutContext);
  const compact = layout?.compact ?? false;
  return (
    <>
      <div
        data-sidebar-section-header={section}
        data-section-label={label}
        inert={compact}
        aria-hidden={compact || undefined}
        className={cx(
          className,
          css.ExpandableListHeaderWrapper,
          compact && css.MeasurementHeader
        )}
      >
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

      {compact &&
        isExpanded &&
        layout?.actions &&
        createPortal(renderHeaderAction?.(), layout.actions)}

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
        <Collapse in={isExpanded} timeout={compact ? 0 : undefined}>
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
