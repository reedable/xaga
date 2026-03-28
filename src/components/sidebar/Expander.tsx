import { ChevronRight, RssFeed } from "@mui/icons-material";
import {
  CircularProgress,
  Collapse,
  List,
  ListItem,
  ListItemButton,
  ListItemIcon,
  ListItemText,
  ToggleButton,
  Typography,
} from "@mui/material";
import { useCallback, type ReactNode } from "react";
import type { Item } from "../../types";
import cx from "../../utils/cx";
import css from "./_.module.css";
import { useFeeds } from "../../hooks/useFeeds";
import useAbortController from "../../hooks/useAbortController";

export interface ExpanderProps {
  label: string;
  items: Item[];
  isExpanded: boolean;
  selectedList: Item | null;
  onClickHeader: () => void;
  onSelectList: (item: Item) => void;
  renderHeaderAction?: () => ReactNode;
  renderSecondaryAction?: (item: Item) => ReactNode;
}

export default function Expander({
  label,
  items,
  isExpanded,
  selectedList,
  onClickHeader,
  onSelectList,
  renderHeaderAction,
  renderSecondaryAction,
}: ExpanderProps) {
  const [, abortControllerRef] = useAbortController();
  const { loadingSet } = useFeeds(abortControllerRef);

  const handleClickHeader = useCallback(() => {
    onClickHeader();
  }, [onClickHeader]);

  return (
    <>
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

        {renderHeaderAction && (
          <span onClick={(e) => e.stopPropagation()}>
            {renderHeaderAction()}
          </span>
        )}
      </ToggleButton>

      <div className={cx(css.ExpanderContent, isExpanded && css.Expanded)}>
        <Collapse in={isExpanded}>
          <List dense disablePadding>
            {items.map((item) => (
              <ListItem
                key={item.id}
                disablePadding
                secondaryAction={renderSecondaryAction?.(item)}
              >
                <ListItemButton
                  selected={selectedList?.id === item.id}
                  onClick={() => onSelectList(item)}
                >
                  <ListItemIcon sx={{ minWidth: 32 }}>
                    {loadingSet?.has(item.id) ? (
                      <CircularProgress size={16} />
                    ) : (
                      <RssFeed sx={{ fontSize: 18, color: "text.secondary" }} />
                    )}
                  </ListItemIcon>

                  <ListItemText
                    primary={item.title}
                    primaryTypographyProps={{ variant: "body2", noWrap: true }}
                  />
                </ListItemButton>
              </ListItem>
            ))}
          </List>
        </Collapse>
      </div>
    </>
  );
}
