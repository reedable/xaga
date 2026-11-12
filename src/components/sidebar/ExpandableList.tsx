import { ChevronRight } from "@mui/icons-material";
import { Collapse, List, ToggleButton, Typography } from "@mui/material";
import {
  Children,
  isValidElement,
  useContext,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import { Virtuoso, type ItemProps, type VirtuosoHandle } from "react-virtuoso";
import cx from "../../utils/cx";
import css from "./ExpandableList.module.css";
import { SidebarListLayoutContext } from "./SidebarListLayoutContext";

// Keep the measured wrapper as the list item; row content can contain multiple controls.
function VirtualItem({
  item: _item,
  context,
  ...props
}: ItemProps<ReactNode> & { context?: { itemCount: number } }) {
  return (
    <div
      {...props}
      role="listitem"
      aria-posinset={props["data-index"] + 1}
      aria-setsize={context?.itemCount}
    />
  );
}

const virtualComponents = { Item: VirtualItem };

function focusableControls(row: Element) {
  return [
    ...row.querySelectorAll<HTMLElement>(
      "button, a[href], input, select, textarea, [tabindex]"
    ),
  ].filter(
    (control) =>
      control.tabIndex >= 0 &&
      !control.matches(":disabled") &&
      control.getClientRects().length > 0
  );
}

export interface ExpandableListProps {
  children?: ReactNode;
  className?: string;
  count: number;
  label: string;
  section?: string;
  /** Nested trees manage their own rendering and can opt out. */
  virtualized?: boolean;
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
  virtualized = true,
  isExpanded,
  onClickHeader,
  renderHeaderAction,
}: ExpandableListProps) {
  const layout = useContext(SidebarListLayoutContext);
  const compact = layout?.compact ?? false;
  const [scrollParent, setScrollParent] = useState<HTMLDivElement | null>(null);
  const virtuosoRef = useRef<VirtuosoHandle>(null);
  const rows = useMemo(() => Children.toArray(children), [children]);
  const virtualContext = useMemo(
    () => ({ itemCount: rows.length }),
    [rows.length]
  );

  function handleKeyDown(event: KeyboardEvent<HTMLElement>) {
    if (
      !virtualized ||
      event.key !== "Tab" ||
      event.altKey ||
      event.ctrlKey ||
      event.metaKey
    )
      return;
    const target = event.target as HTMLElement;
    const row = target.closest<HTMLElement>("[data-item-index]");
    if (!row) return;
    const controls = focusableControls(row);
    const boundary = event.shiftKey ? controls[0] : controls.at(-1);
    if (target !== boundary) return;

    const direction = event.shiftKey ? -1 : 1;
    let index = Number(row.dataset.itemIndex) + direction;
    // Loading/status rows have no controls and must not interrupt Tab traversal.
    while (index >= 0 && index < rows.length) {
      const next = scrollParent?.querySelector(`[data-item-index="${index}"]`);
      if (!next || focusableControls(next).length > 0) break;
      index += direction;
    }
    if (index < 0 || index >= rows.length) return;

    event.preventDefault();
    virtuosoRef.current?.scrollIntoView({
      index,
      behavior: "auto",
      done: () => {
        const next = scrollParent?.querySelector(
          `[data-item-index="${index}"]`
        );
        if (!next) return;
        const nextControls = focusableControls(next);
        (event.shiftKey ? nextControls.at(-1) : nextControls[0])?.focus();
      },
    });
  }
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
        ref={setScrollParent}
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
            component={virtualized ? "div" : "ul"}
            role="list"
            onKeyDown={handleKeyDown}
            data-testid={section && `sidebar-${section}-list`}
            aria-label={label}
            dense
            disablePadding
          >
            {!virtualized
              ? children
              : scrollParent &&
                rows.length > 0 && (
                  <Virtuoso
                    ref={virtuosoRef}
                    components={virtualComponents}
                    context={virtualContext}
                    customScrollParent={scrollParent}
                    data={rows}
                    computeItemKey={(index, row) =>
                      isValidElement(row) ? (row.key ?? index) : index
                    }
                    itemContent={(_index, row) => row}
                    increaseViewportBy={100}
                  />
                )}
          </List>
        </Collapse>
      </div>
    </>
  );
}
