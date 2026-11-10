import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Typography,
} from "@mui/material";
import { useId } from "react";
import type { FeedRefreshFailure } from "../../types";
import css from "./FeedRefreshFailuresDialog.module.css";

export default function FeedRefreshFailuresDialog({
  failures,
  onClose,
}: {
  failures: readonly FeedRefreshFailure[];
  onClose: () => void;
}) {
  const titleId = useId();
  return (
    <Dialog
      open
      transitionDuration={0}
      slotProps={{
        backdrop: { transitionDuration: 0 },
        paper: { className: css.Paper },
      }}
      onClose={onClose}
      aria-labelledby={titleId}
      maxWidth="sm"
      fullWidth
    >
      <DialogTitle id={titleId}>Feeds not refreshed</DialogTitle>
      <DialogContent dividers className={css.Content}>
        <ul className={css.Failures}>
          {failures.map((failure) => (
            <li key={failure.id} data-feed-id={failure.id}>
              <Typography component="h3" variant="subtitle2">
                {failure.title}
              </Typography>
              <Typography className={css.Reason} variant="body2">
                {failure.message}
              </Typography>
            </li>
          ))}
        </ul>
      </DialogContent>
      <DialogActions>
        <Button
          className={css.Close}
          data-testid="close-feed-failures"
          onClick={onClose}
        >
          Close
        </Button>
      </DialogActions>
    </Dialog>
  );
}
