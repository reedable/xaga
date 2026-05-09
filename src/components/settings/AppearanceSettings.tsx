import {
  DensityLarge,
  DensitySmall,
  TextDecrease,
  TextIncrease,
} from "@mui/icons-material";
import {
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
import type { FontFamily } from "../../utils/FontFamily";
import {
  READER_FONT_SIZE_MAX,
  READER_FONT_SIZE_MIN,
  READER_LINE_HEIGHT_MAX,
  READER_LINE_HEIGHT_MIN,
  READER_LINE_HEIGHT_STEP,
  type ThemeSetting,
} from "../../providers/AppearanceContext";
import css from "./AppearanceSettings.module.css";

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
    <div className={css.Stepper}>
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
      <Typography variant="body2" className={css.StepperLabel}>
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
    </div>
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
    <div>
      <Typography component="h1" variant="h6" gutterBottom>
        Appearance
      </Typography>

      <div className={css.ThemeSection}>
        <FormControl size="small" className={css.SelectControl}>
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
      </div>

      <div className={css.SummarySection}>
        <FormControlLabel
          control={
            <Checkbox
              checked={view === "summary"}
              onChange={(e) => setView(e.target.checked ? "summary" : "full")}
            />
          }
          label="Show Article Summary"
        />
      </div>

      <Typography
        component="h2"
        variant="subtitle1"
        className={css.ReadingHeading}
      >
        Reading
      </Typography>

      <div className={css.FontSection}>
        <FormControl size="small" className={css.SelectControl}>
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
      </div>

      <div className={css.StepperRow}>
        <Typography
          variant="body2"
          color="text.secondary"
          className={css.StepperRowLabel}
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
      </div>

      <div className={css.StepperRow}>
        <Typography
          variant="body2"
          color="text.secondary"
          className={css.StepperRowLabel}
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
      </div>
    </div>
  );
}
