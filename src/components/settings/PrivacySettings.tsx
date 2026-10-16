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
import { usePlatform } from "../../platform/Platform";
import css from "./PrivacySettings.module.css";

const DOMAIN_RE =
  /^[a-z0-9]([a-z0-9-]*[a-z0-9])?(\.[a-z0-9]([a-z0-9-]*[a-z0-9])?)+$/i;

export function PrivacySettings() {
  const platform = usePlatform();
  const [captureImages, setCaptureImages] = useState(false);
  const [captureLoaded, setCaptureLoaded] = useState(false);
  const [captureSaving, setCaptureSaving] = useState(false);
  const [captureError, setCaptureError] = useState<string | null>(null);
  const [domains, setDomains] = useState<string[]>([]);
  const [input, setInput] = useState("");
  const [inputError, setInputError] = useState(false);

  useEffect(() => {
    platform.getAllowedDomains().then(setDomains);
  }, [platform]);

  useEffect(() => {
    let active = true;
    platform.getLoadRemoteImage().then((value) => {
      if (active) { setCaptureImages(value); setCaptureLoaded(true); }
    }).catch(() => {
      if (active) setCaptureError("Could not load the image capture preference. Reopen Settings to retry.");
    });
    return () => { active = false; };
  }, [platform]);

  async function handleCaptureChange(value: boolean) {
    setCaptureSaving(true);
    setCaptureError(null);
    try {
      await platform.setLoadRemoteImage(value);
      setCaptureImages(value);
    } catch {
      setCaptureError("Could not save the image capture preference. Please try again.");
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
    setDomains(next);
    setInput("");
    setInputError(false);
    await platform.setAllowedDomains(next);
  }

  async function handleRemove(domain: string) {
    const next = domains.filter((d) => d !== domain);
    setDomains(next);
    await platform.setAllowedDomains(next);
  }

  return (
    <div>
      <Typography component="h1" variant="h6" gutterBottom>
        Privacy
      </Typography>

      <div className={css.Section}>
        <FormControlLabel
          control={<Switch checked={captureImages} disabled={!captureLoaded || captureSaving}
            onChange={(_, checked) => void handleCaptureChange(checked)}
            slotProps={{ input: { "aria-describedby": "image-capture-description" } }} />}
          label="Save article images for offline viewing"
        />
        <Typography id="image-capture-description" variant="body2" color="text.secondary">
          Off by default. Enabling this allows additional network requests to save
          images when loading articles and recover unavailable images when opening
          saved articles. Changes apply to new loads; reopen existing loader windows
          to use the new preference. Saved images remain available when this is off.
          Original pages can still display images in the loader window.
        </Typography>
        {captureError && <Alert severity="error">{captureError}</Alert>}
      </div>

      <div className={css.Section}>
        <Typography variant="subtitle2" gutterBottom>
          Allowed Domains
        </Typography>
        <Typography variant="body2" color="text.secondary" gutterBottom>
          By default, every browsing session is incognito — no cookies or site
          data are saved. Add domains here to keep persistent sessions (e.g. to
          stay logged in). Each domain gets its own isolated cookie jar.
          Persistent sessions require macOS 14 or later; on older systems, all
          sessions remain incognito.
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
