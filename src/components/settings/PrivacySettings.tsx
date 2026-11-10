import { SettingsControlLabel } from "./SettingsControlLabel";
import { Close } from "@mui/icons-material";
import {
  Alert,
  FormControlLabel,
  Switch,
  Button,
  Chip,
  IconButton,
  TextField,
  Tooltip,
  Typography,
} from "@mui/material";
import { useEffect, useState } from "react";
import { useSettings } from "../../hooks/useSettings";
import { useErrorSink } from "../../hooks/useErrorSink";
import { usePlatform } from "../../platform/Platform";
import css from "./PrivacySettings.module.css";

import { FetchSwitch } from "./FetchSettingControls";

const DOMAIN_RE =
  /^[a-z0-9]([a-z0-9-]*[a-z0-9])?(\.[a-z0-9]([a-z0-9-]*[a-z0-9])?)+$/i;

export function PrivacySettings() {
  const platform = usePlatform();
  const { saveSettings, refreshVersion } = useSettings();
  const { pushError } = useErrorSink();
  const [captureImages, setCaptureImages] = useState(false);
  const [captureLoaded, setCaptureLoaded] = useState(false);
  const [captureSaving, setCaptureSaving] = useState(false);
  const [captureError, setCaptureError] = useState<string | null>(null);
  const [domains, setDomains] = useState<string[]>([]);
  const [input, setInput] = useState("");
  const [inputError, setInputError] = useState(false);

  useEffect(() => {
    platform.getAllowedDomains().then(setDomains).catch(pushError);
  }, [platform, pushError, refreshVersion]);

  useEffect(() => {
    let active = true;
    platform
      .getLoadRemoteImage()
      .then((value) => {
        if (active) {
          setCaptureImages(value);
          setCaptureLoaded(true);
          setCaptureError(null);
        }
      })
      .catch(() => {
        if (active)
          setCaptureError(
            "Could not load the image capture preference. Reopen Settings to retry."
          );
      });
    return () => {
      active = false;
    };
  }, [platform, pushError, refreshVersion]);

  async function handleCaptureChange(value: boolean) {
    setCaptureSaving(true);
    setCaptureError(null);
    try {
      const saved = await saveSettings(() =>
        platform.setLoadRemoteImage(value)
      );
      if (saved) setCaptureImages(value);
    } catch {
      setCaptureError(
        "Could not save the image capture preference. Please try again."
      );
    } finally {
      setCaptureSaving(false);
    }
  }

  async function handleAdd() {
    const trimmed = input.trim().toLowerCase();
    if (!DOMAIN_RE.test(trimmed) || domains.includes(trimmed)) {
      setInputError(true);
      return;
    }
    const next = [...domains, trimmed];
    if (await saveSettings(() => platform.setAllowedDomains(next))) {
      setDomains(next);
      setInput("");
      setInputError(false);
    }
  }

  async function handleRemove(domain: string) {
    const next = domains.filter((d) => d !== domain);
    if (await saveSettings(() => platform.setAllowedDomains(next)))
      setDomains(next);
  }

  return (
    <div>
      <Typography component="h1" variant="h6" gutterBottom>
        Privacy
      </Typography>

      <FetchSwitch
        setting="refererAllowed"
        label="Send referring URLs for feed redirects"
        description="Off by default. Redirect destinations can learn the previous feed URL when enabled. May help feeds that require a referring URL. Applies to the next feed add, refresh, or import; does not change article-loader headers."
      />
      <FetchSwitch
        setting="gzipAllowed"
        label="Allow compressed feed transfers"
        description="Off by default. Advertises gzip support to feed websites, revealing another client capability in exchange for potentially smaller transfers. The feed size limit still counts decompressed bytes. Applies to the next feed add, refresh, or import; does not change article-loader compression."
      />

      <div className={css.Section}>
        <FormControlLabel
          control={
            <Switch
              checked={captureImages}
              disabled={!captureLoaded || captureSaving}
              onChange={(_, checked) => void handleCaptureChange(checked)}
              slotProps={{
                input: {
                  "aria-labelledby": "image-capture-description-label",
                  "aria-describedby": "image-capture-description",
                },
              }}
            />
          }
          disableTypography
          sx={{ alignItems: "flex-start" }}
          label={
            <SettingsControlLabel
              id="image-capture-description"
              label="Save article images for offline viewing"
            >
              Off by default. Enabling this allows additional network requests
              to save images when loading articles and recover unavailable
              images when opening saved articles. Changes apply to new loads;
              reopen existing loader windows to use the new preference. Saved
              images remain available when this is off. Original pages can still
              display images in the loader window.
            </SettingsControlLabel>
          }
        />
        {captureError && <Alert severity="error">{captureError}</Alert>}
      </div>

      <div className={css.Section}>
        <Typography
          data-testid="website-sessions"
          variant="subtitle2"
          gutterBottom
        >
          Keep website sessions
        </Typography>
        <Typography variant="body2" color="text.secondary" gutterBottom>
          By default, every browsing session is incognito — no cookies or site
          data are saved. Add domains here to keep persistent sessions (e.g. to
          stay logged in). Each domain gets its own isolated cookie jar.
          Persistent sessions require macOS 14 or later; on older systems, all
          sessions remain incognito. Changes apply when opening a new loader
          window; reopen existing windows to use the new session preference.
        </Typography>

        <div className={css.InputRow}>
          <TextField
            size="small"
            placeholder="example.com"
            value={input}
            error={inputError}
            helperText={
              inputError
                ? "Enter a valid domain not already in the list."
                : undefined
            }
            onChange={(e) => {
              setInput(e.target.value);
              setInputError(false);
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleAdd();
            }}
            className={css.DomainInput}
          />
          <Button
            data-testid="add"
            variant="outlined"
            onClick={handleAdd}
            className={css.AddButton}
          >
            Add
          </Button>
        </div>

        {domains.length > 0 && (
          <div className={css.ChipList}>
            {domains.map((domain) => (
              <Chip
                key={domain}
                label={domain}
                deleteIcon={
                  <Tooltip title={`Remove ${domain}`}>
                    <IconButton size="small">
                      <Close fontSize="inherit" />
                    </IconButton>
                  </Tooltip>
                }
                onDelete={() => handleRemove(domain)}
                variant="outlined"
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
