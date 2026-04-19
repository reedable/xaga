import {
  DensityLarge,
  DensitySmall,
  TextDecrease,
  TextIncrease,
} from "@mui/icons-material";
import {
  Box,
  Checkbox,
  FormControl,
  FormControlLabel,
  IconButton,
  InputLabel,
  MenuItem,
  Select,
  Tooltip,
  Typography,
} from "@mui/material";
import type { ReactNode } from "react";
import { useAppearance } from "../../hooks/useAppearance";
import {
  READER_FONT_SIZE_MAX,
  READER_FONT_SIZE_MIN,
  READER_LINE_HEIGHT_MAX,
  READER_LINE_HEIGHT_MIN,
  READER_LINE_HEIGHT_STEP,
  type FontFamily,
  type ThemeSetting,
} from "../../providers/AppearanceContext";

interface StepperProps {
  value: number;
  display: string;
  min: number;
  max: number;
  step: number;
  decrementIcon: ReactNode;
  incrementIcon: ReactNode;
  decrementLabel: string;
  incrementLabel: string;
  onChange: (value: number) => void;
}

function ReaderStepper({
  value,
  display,
  min,
  max,
  step,
  decrementIcon,
  incrementIcon,
  decrementLabel,
  incrementLabel,
  onChange,
}: StepperProps) {
  return (
    <Box
      sx={{
        alignItems: "center",
        border: "1px solid",
        borderColor: "divider",
        borderRadius: 1,
        display: "flex",
        width: "fit-content",
      }}
    >
      <Tooltip title={decrementLabel}>
        <span>
          <IconButton
            size="small"
            disabled={value <= min}
            onClick={() => onChange(Math.round((value - step) * 10) / 10)}
          >
            {decrementIcon}
          </IconButton>
        </span>
      </Tooltip>
      <Typography
        variant="body2"
        sx={{
          minWidth: 52,
          px: 1,
          textAlign: "center",
          userSelect: "none",
        }}
      >
        {display}
      </Typography>
      <Tooltip title={incrementLabel}>
        <span>
          <IconButton
            size="small"
            disabled={value >= max}
            onClick={() => onChange(Math.round((value + step) * 10) / 10)}
          >
            {incrementIcon}
          </IconButton>
        </span>
      </Tooltip>
    </Box>
  );
}

export function AppearanceSettings() {
  const {
    themeSetting,
    changeTheme,
    readerView: view,
    setView,
    readerFontFamily,
    setReaderFontFamily,
    readerFontSize,
    setReaderFontSize,
    readerLineHeight,
    setReaderLineHeight,
  } = useAppearance();

  return (
    <Box>
      <Typography component="h1" variant="h6" gutterBottom>
        Appearance
      </Typography>

      <Box sx={{ mt: 3 }}>
        <FormControl size="small" sx={{ minWidth: 200 }}>
          <InputLabel>Theme</InputLabel>
          <Select
            value={themeSetting}
            label="Theme"
            onChange={(e) => changeTheme(e.target.value as ThemeSetting)}
          >
            <MenuItem value="auto">Auto (System)</MenuItem>
            <MenuItem value="dark">Dark</MenuItem>
            <MenuItem value="light">Light</MenuItem>
          </Select>
        </FormControl>
      </Box>

      <Box sx={{ mt: 2 }}>
        <FormControlLabel
          control={
            <Checkbox
              checked={view === "summary"}
              onChange={(e) => setView(e.target.checked ? "summary" : "full")}
            />
          }
          label="Show Article Summary"
        />
      </Box>

      <Typography
        component="h2"
        variant="subtitle1"
        sx={{ mt: 4, mb: 1, fontWeight: 600 }}
      >
        Reading
      </Typography>

      <Box sx={{ mt: 1 }}>
        <FormControl size="small" sx={{ minWidth: 200 }}>
          <InputLabel>Font</InputLabel>
          <Select
            value={readerFontFamily}
            label="Font"
            onChange={(e) => setReaderFontFamily(e.target.value as FontFamily)}
          >
            <MenuItem value="system">System UI</MenuItem>
            <MenuItem value="serif">Serif</MenuItem>
            <MenuItem value="sans-serif">Sans-serif</MenuItem>
            <MenuItem value="monospace">Monospace</MenuItem>
          </Select>
        </FormControl>
      </Box>

      <Box sx={{ alignItems: "center", display: "flex", gap: 2, mt: 2 }}>
        <Typography
          variant="body2"
          color="text.secondary"
          sx={{ minWidth: 80 }}
        >
          Font size
        </Typography>
        <ReaderStepper
          value={readerFontSize}
          display={`${readerFontSize}px`}
          min={READER_FONT_SIZE_MIN}
          max={READER_FONT_SIZE_MAX}
          step={1}
          decrementIcon={<TextDecrease fontSize="small" />}
          incrementIcon={<TextIncrease fontSize="small" />}
          decrementLabel="Decrease font size"
          incrementLabel="Increase font size"
          onChange={setReaderFontSize}
        />
      </Box>

      <Box sx={{ alignItems: "center", display: "flex", gap: 2, mt: 2 }}>
        <Typography
          variant="body2"
          color="text.secondary"
          sx={{ minWidth: 80 }}
        >
          Line height
        </Typography>
        <ReaderStepper
          value={readerLineHeight}
          display={readerLineHeight.toFixed(1)}
          min={READER_LINE_HEIGHT_MIN}
          max={READER_LINE_HEIGHT_MAX}
          step={READER_LINE_HEIGHT_STEP}
          decrementIcon={<DensitySmall fontSize="small" />}
          incrementIcon={<DensityLarge fontSize="small" />}
          decrementLabel="Tighten line height"
          incrementLabel="Loosen line height"
          onChange={setReaderLineHeight}
        />
      </Box>
    </Box>
  );
}
