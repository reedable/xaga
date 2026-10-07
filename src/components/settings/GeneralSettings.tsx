import {
  Button,
  Checkbox,
  FormControlLabel,
  TextField,
  Typography,
} from "@mui/material";
import { useSettings } from "../../hooks/useSettings";
import css from "./GeneralSettings.module.css";

export function GeneralSettings() {
  const {
    library,
    switchLibrary,
    refreshOnStartup,
    setRefreshOnStartup,
    inlinePdfReader,
    setInlinePdfReader,
  } = useSettings();

  return (
    <div>
      <Typography component="h1" variant="h6" gutterBottom>
        General
      </Typography>

      <div className={css.LibrarySection}>
        <Typography variant="subtitle2" gutterBottom>
          Library Location
        </Typography>

        <div className={css.LibraryRow}>
          <TextField
            value={library}
            size="small"
            slotProps={{ input: { readOnly: true } }}
            fullWidth
          />

          <Button
            variant="outlined"
            onClick={switchLibrary}
            className={css.SwitchButton}
          >
            Switch
          </Button>
        </div>
      </div>

      <div className={css.RefreshSection}>
        <FormControlLabel
          control={
            <Checkbox
              checked={inlinePdfReader}
              onChange={(e) => setInlinePdfReader(e.target.checked)}
            />
          }
          label="Show PDFs in the reading pane"
        />
        <Typography variant="body2" color="text.secondary">
          When off, PDFs open in your default app.
        </Typography>
      </div>

      <div className={css.RefreshSection}>
        <FormControlLabel
          control={
            <Checkbox
              checked={refreshOnStartup}
              onChange={(e) => setRefreshOnStartup(e.target.checked)}
            />
          }
          label="Refresh all feeds on start-up"
        />
      </div>
    </div>
  );
}
