import { Close } from "@mui/icons-material";
import {
  Box,
  Button,
  Checkbox,
  FormControlLabel,
  IconButton,
  TextField,
  Tooltip,
  Typography,
} from "@mui/material";
import { useNavigate } from "react-router-dom";
import { useSettings } from "../../hooks/useSettings";

export function GeneralSettings() {
  const navigate = useNavigate();

  const { library, switchLibrary, refreshOnStartup, setRefreshOnStartup } =
    useSettings();

  return (
    <Box>
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          flexWrap: "nowrap",
        }}
      >
        <Typography component="h1" variant="h6" gutterBottom>
          General
        </Typography>

        <Tooltip title="Close">
          <IconButton aria-label="Close" onClick={() => navigate("/")}>
            <Close />
          </IconButton>
        </Tooltip>
      </div>

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
