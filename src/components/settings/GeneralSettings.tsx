import { SettingsControlLabel } from "./SettingsControlLabel";
import {
  Button,
  Checkbox,
  FormControlLabel,
  TextField,
  Typography,
} from "@mui/material";
import { useEffect, useId, useState } from "react";
import { usePlatform } from "../../platform/Platform";
import { useErrorSink } from "../../hooks/useErrorSink";
import { NumericSetting } from "./NumericSetting";
import { FetchNumber } from "./FetchSettingControls";
import { useSettings } from "../../hooks/useSettings";
import css from "./GeneralSettings.module.css";

export function GeneralSettings() {
  const pdfHelpId = useId();
  const refreshHelpId = useId();
  const {
    library,
    switchLibrary,
    refreshOnStartup,
    setRefreshOnStartup,
    inlinePdfReader,
    setInlinePdfReader,
  } = useSettings();

  return (
    <div className={css.General}>
      <Typography component="h1" variant="h6" gutterBottom>
        General
      </Typography>

      <div className={css.LibrarySection}>
        <Typography variant="subtitle2" gutterBottom>
          Library location
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
            Switch library
          </Button>
        </div>
      </div>

      <Typography
        component="h2"
        variant="subtitle1"
        className={css.SectionHeading}
      >
        Size limits
      </Typography>
      <Typography
        variant="body2"
        color="text.secondary"
        className={css.SectionDescription}
      >
        Larger inputs can use more memory. Changes apply to the next import or
        feed request.
      </Typography>
      <ImportSizeSetting />
      <FetchNumber
        setting="feedMaxBodyBytes"
        label="Maximum feed response size"
        unit="MiB"
        description="Feed downloads; 10 MiB by default. Counts decompressed bytes."
      />
      <Typography
        component="h2"
        variant="subtitle1"
        className={css.SectionHeading}
      >
        Timeouts
      </Typography>
      <Typography
        variant="body2"
        color="text.secondary"
        className={css.SectionDescription}
      >
        Longer waits allow slow websites more time but delay failures. Changes
        apply to new article loads and feed requests.
      </Typography>
      <FetchNumber
        setting="loaderTimeoutSecs"
        label="Article load timeout"
        unit="sec"
        description="Article loading and image recovery. Default: 10 seconds."
      />
      <FetchNumber
        setting="feedTimeoutSecs"
        label="Feed request timeout"
        unit="sec"
        description="Connection, response, and body reading. Default: 10 seconds."
      />

      <div className={css.RefreshSection}>
        <FormControlLabel
          control={
            <Checkbox
              slotProps={{
                input: {
                  "aria-labelledby": `${pdfHelpId}-label`,
                  "aria-describedby": pdfHelpId,
                },
              }}
              checked={inlinePdfReader}
              onChange={(e) => setInlinePdfReader(e.target.checked)}
            />
          }
          disableTypography
          sx={{ alignItems: "flex-start" }}
          label={
            <SettingsControlLabel
              id={pdfHelpId}
              label="Show PDFs in the reading pane"
            >
              When off, PDFs open in your default app.
            </SettingsControlLabel>
          }
        />
      </div>

      <div className={css.RefreshSection}>
        <FormControlLabel
          control={
            <Checkbox
              slotProps={{
                input: {
                  "aria-labelledby": `${refreshHelpId}-label`,
                  "aria-describedby": refreshHelpId,
                },
              }}
              checked={refreshOnStartup}
              onChange={(e) => setRefreshOnStartup(e.target.checked)}
            />
          }
          disableTypography
          sx={{ alignItems: "flex-start" }}
          label={
            <SettingsControlLabel
              id={refreshHelpId}
              label="Refresh all feeds on start-up"
            >
              Contacts subscribed websites automatically on the next startup.
            </SettingsControlLabel>
          }
        />
      </div>
    </div>
  );
}

const MIB = 1024 * 1024;

function ImportSizeSetting() {
  const platform = usePlatform();
  const { saveSettings, refreshVersion } = useSettings();
  const { pushError } = useErrorSink();
  const [savedBytes, setSavedBytes] = useState<number | null>(null);
  useEffect(() => {
    let active = true;
    platform
      .getImportMaxBytes()
      .then((limit) => {
        if (active) setSavedBytes(limit);
      })
      .catch((error) => {
        if (active) pushError(error);
      });
    return () => {
      active = false;
    };
  }, [platform, pushError, refreshVersion]);
  return (
    <NumericSetting
      label="Maximum import file size"
      savedValue={savedBytes === null ? null : String(savedBytes / MIB)}
      minimum={1n}
      maximum={BigInt(Math.floor(Number.MAX_SAFE_INTEGER / MIB))}
      unit="MiB"
      description="Feed and bookmark import files."
      saveValue={async (value) => {
        const bytes = Number(value) * MIB;
        const saved = await saveSettings(() =>
          platform.setImportMaxBytes(bytes)
        );
        if (saved) setSavedBytes(bytes);
        return saved;
      }}
    />
  );
}
