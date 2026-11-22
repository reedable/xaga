import { useId } from "react";
import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  List,
  ListItem,
  ListItemButton,
  ListItemText,
} from "@mui/material";
import type { Topic } from "../../types";

export default function SimilarTopicsDialog({
  matches,
  onOpenTopic,
  onDismiss,
  onConfirm,
}: {
  matches: Topic[] | null;
  onOpenTopic: (topic: Topic) => void;
  onDismiss: () => void;
  onConfirm: () => void;
}) {
  const titleId = useId();

  return (
    <Dialog
      data-testid="similar-topics-warning"
      open
      onClose={onDismiss}
      aria-labelledby={titleId}
      fullWidth
      maxWidth="sm"
    >
      <DialogTitle id={titleId}>
        {matches ? "Similar Topics" : "Similarity check unavailable"}
      </DialogTitle>
      <DialogContent>
        <DialogContentText>
          {matches
            ? "Open a similar Topic, revise your draft, or create a new Topic."
            : "Xaga couldn’t check for similar Topics. You can still create this Topic."}
        </DialogContentText>
        {matches && (
          <List aria-labelledby={titleId}>
            {matches.map((topic) => (
              <ListItem key={topic.id} disablePadding>
                <ListItemButton
                  component="button"
                  type="button"
                  data-testid="open-similar-topic"
                  onClick={() => onOpenTopic(topic)}
                >
                  <ListItemText
                    primary={topic.title}
                    secondary={topic.description}
                  />
                </ListItemButton>
              </ListItem>
            ))}
          </List>
        )}
      </DialogContent>
      <DialogActions>
        <Button data-testid="revise-topic" onClick={onDismiss} autoFocus>
          Revise draft
        </Button>
        <Button
          data-testid="create-topic-anyway"
          onClick={onConfirm}
          variant="contained"
        >
          Create anyway
        </Button>
      </DialogActions>
    </Dialog>
  );
}
