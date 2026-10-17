import {
  Button,
  Checkbox,
  FormControlLabel,
  TextField,
  Typography,
} from "@mui/material";
import { useEffect, useEffectEvent, useRef, useState } from "react";
import { usePlatform } from "../../platform/Platform";
import { useErrorSink } from "../../hooks/useErrorSink";
import { subscribe } from "../../hooks/useEvents";
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

      <ImportSizeSetting />

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

const MIB = 1024 * 1024;

function ImportSizeSetting() {
  const platform = usePlatform();
  const { saveSettings, refreshVersion } = useSettings();
  const { pushError } = useErrorSink();
  const [savedBytes, setSavedBytes] = useState<number | null>(null);
  const [draft, setDraft] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const pendingSave = useRef<Promise<boolean> | null>(null);
  const value = draft ?? (savedBytes === null ? "" : String(savedBytes / MIB));
  const bytes = Number(value) * MIB;
  const valid =
    value.trim() !== "" &&
    Number.isInteger(Number(value)) &&
    Number.isSafeInteger(bytes) &&
    bytes > 0;

  useEffect(() => {
    let active = true;
    platform
      .getImportMaxBytes()
      .then((limit) => {
        if (active) {
          setSavedBytes(limit);
          setDraft(null);
        }
      })
      .catch((error) => {
        if (active) {
          setSavedBytes(null);
          pushError(error);
        }
      });
    return () => {
      active = false;
    };
  }, [platform, pushError, refreshVersion]);

  function save(): Promise<boolean> {
    if (pendingSave.current) return pendingSave.current;
    if (draft === null || bytes === savedBytes) return Promise.resolve(true);
    if (!valid || savedBytes === null) return Promise.resolve(false);
    setSaving(true);
    const pending = saveSettings(() => platform.setImportMaxBytes(bytes))
      .then((saved) => {
        if (saved) {
          setSavedBytes(bytes);
          setDraft(null);
        }
        return saved;
      })
      .finally(() => {
        pendingSave.current = null;
        setSaving(false);
      });
    pendingSave.current = pending;
    return pending;
  }

  const saveBeforeClose = useEffectEvent(() => save());
  useEffect(
    () =>
      subscribe(() =>
        platform.onWindowCloseRequested(saveBeforeClose).catch((error) => {
          pushError(error);
          return () => {};
        })
      ),
    [platform, pushError]
  );

  return (
    <div className={css.RefreshSection}>
      <TextField
        label="Maximum import file size"
        type="number"
        size="small"
        value={value}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={() => void save()}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            void save();
          }
        }}
        disabled={savedBytes === null || saving}
        error={draft !== null && !valid}
        helperText={
          draft !== null && !valid
            ? "Enter a positive whole number of MiB."
            : "MiB (1 MiB = 1,048,576 bytes)"
        }
        slotProps={{
          htmlInput: {
            min: 1,
            step: 1,
            max: Math.floor(Number.MAX_SAFE_INTEGER / MIB),
          },
        }}
      />
      <Typography variant="body2" color="text.secondary">
        Applies to feed and bookmark imports. Raising the limit allows larger
        files and may increase memory use. Changes apply to the next import.
      </Typography>
    </div>
  );
}
