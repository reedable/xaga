import { LinkOff } from "@mui/icons-material";
import { IconButton, ListItem, ListItemText, Tooltip } from "@mui/material";
import type { PinTarget } from "../../pins";
import { usePins } from "../../providers/PinsContext";
import { useLibraryReadOnly } from "../../hooks/useLibraryReadOnly";
import { useErrorSink } from "../../hooks/useErrorSink";

export default function MissingShortcut({ target }: { target: PinTarget }) {
  const { setPin } = usePins();
  const readOnly = useLibraryReadOnly();
  const { pushError } = useErrorSink();
  return (
    <ListItem
      secondaryAction={
        <Tooltip title="Remove from Shortcuts">
          <span>
            <IconButton
              disabled={readOnly}
              aria-label={`Remove ${target.type} ${target.id} from Shortcuts`}
              onClick={(event) => {
                event.stopPropagation();
                void setPin(target, false).catch(pushError);
              }}
            >
              <LinkOff />
            </IconButton>
          </span>
        </Tooltip>
      }
    >
      <ListItemText
        primary={`${target.type}: ${target.id}`}
        secondary="Target cannot be found"
      />
    </ListItem>
  );
}
