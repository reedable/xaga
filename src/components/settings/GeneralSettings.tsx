import {
  Box,
  Button,
  Checkbox,
  FormControlLabel,
  TextField,
  Typography,
} from "@mui/material";
import { useSettings } from "../../hooks/useSettings";

export function GeneralSettings() {
  const { library, switchLibrary, refreshOnStartup, setRefreshOnStartup } =
    useSettings();

  return (
    <Box>
      <Typography component="h1" variant="h6" gutterBottom>
        General
      </Typography>

      <Box sx={{ mt: 3 }}>
        <Typography variant="subtitle2" gutterBottom>
          Library Location
        </Typography>

        <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
          <TextField
            value={library}
            size="small"
            slotProps={{ input: { readOnly: true } }}
            fullWidth
          />

          <Button
            variant="outlined"
            onClick={switchLibrary}
            sx={{ whiteSpace: "nowrap" }}
          >
            Switch
          </Button>
        </Box>
      </Box>

      <Box sx={{ mt: 3 }}>
        <FormControlLabel
          control={
            <Checkbox
              checked={refreshOnStartup}
              onChange={(e) => setRefreshOnStartup(e.target.checked)}
            />
          }
          label="Refresh all feeds on start-up"
        />
      </Box>
    </Box>
  );
}
