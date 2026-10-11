import { Close } from "@mui/icons-material";
import {
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
  const [domains, setDomains] = useState<string[]>([]);
  const [input, setInput] = useState("");
  const [inputError, setInputError] = useState(false);

  useEffect(() => {
    platform.getAllowedDomains().then(setDomains);
  }, [platform]);

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
