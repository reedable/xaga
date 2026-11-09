import { InputAdornment, TextField, Typography } from "@mui/material";
import { useId, useRef, useState } from "react";
import { useSaveBeforeSettingsClose } from "./useSaveBeforeSettingsClose";
import css from "./GeneralSettings.module.css";

export function NumericSetting({
  label,
  savedValue,
  saveValue,
  description,
  unit,
  minimum = 0n,
  maximum = 18446744073709551615n,
  conversion,
}: {
  label: string;
  savedValue: string | null;
  saveValue: (value: string) => Promise<boolean>;
  description: string;
  unit: string;
  minimum?: bigint;
  maximum?: bigint;
  conversion?: {
    format: (value: string) => string;
    parse: (value: string) => string | null;
    error: string;
  };
}) {
  const inputId = useId();
  const [draft, setDraft] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const pendingSave = useRef<Promise<boolean> | null>(null);
  const value =
    draft ??
    (savedValue === null
      ? ""
      : conversion
        ? conversion.format(savedValue)
        : savedValue);
  const parsed = conversion
    ? conversion.parse(value)
    : /^\d+$/.test(value)
      ? value
      : null;
  const valid =
    parsed !== null && BigInt(parsed) >= minimum && BigInt(parsed) <= maximum;
  function save(): Promise<boolean> {
    if (pendingSave.current) return pendingSave.current;
    if (draft === null) return Promise.resolve(true);
    if (!valid || savedValue === null) return Promise.resolve(false);
    const normalized = BigInt(parsed!).toString();
    if (normalized === savedValue) {
      setDraft(null);
      return Promise.resolve(true);
    }
    setSaving(true);
    const pending = saveValue(normalized)
      .then((saved) => {
        if (saved) setDraft(null);
        return saved;
      })
      .finally(() => {
        pendingSave.current = null;
        setSaving(false);
      });
    pendingSave.current = pending;
    return pending;
  }
  useSaveBeforeSettingsClose(save);
  return (
    <div className={css.RefreshSection}>
      <Typography
        component="label"
        htmlFor={inputId}
        variant="subtitle2"
        gutterBottom
        className={css.SettingLabel}
      >
        {label}
      </Typography>
      <TextField
        id={inputId}
        size="small"
        fullWidth
        type="text"
        value={value}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={() => void save()}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            void save();
          }
        }}
        disabled={savedValue === null || saving}
        error={draft !== null && !valid}
        helperText={
          draft !== null && !valid
            ? (conversion?.error ??
              `Enter a whole number from ${minimum} to ${maximum}.`)
            : description
        }
        slotProps={{
          input: {
            endAdornment: (
              <InputAdornment position="end">{unit}</InputAdornment>
            ),
          },
          htmlInput: {
            inputMode: conversion ? "decimal" : "numeric",
          },
        }}
      />
    </div>
  );
}
