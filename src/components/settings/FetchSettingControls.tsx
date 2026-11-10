import { SettingsControlLabel } from "./SettingsControlLabel";
import { bytesToMib, mibToBytes } from "./mib";
import { FormControlLabel, Switch } from "@mui/material";
import { useId, useState } from "react";
import type { FetchSettings } from "../../platform/fetchSettings";
import { useFetchSettings } from "./useFetchSettings";
import { NumericSetting } from "./NumericSetting";
import css from "./PrivacySettings.module.css";

type BooleanSetting = {
  [K in keyof FetchSettings]: FetchSettings[K] extends boolean ? K : never;
}[keyof FetchSettings];
type NumberSetting = Exclude<keyof FetchSettings, BooleanSetting>;

export function FetchSwitch({
  setting,
  label,
  description,
}: {
  setting: BooleanSetting;
  label: string;
  description: string;
}) {
  const { settings, save } = useFetchSettings();
  const [saving, setSaving] = useState(false);
  const id = useId();
  return (
    <div className={css.Section}>
      <FormControlLabel
        disableTypography
        sx={{ alignItems: "flex-start" }}
        label={
          <SettingsControlLabel id={id} label={label}>
            {description}
          </SettingsControlLabel>
        }
        control={
          <Switch
            checked={settings?.[setting] ?? false}
            disabled={!settings || saving}
            slotProps={{
              input: {
                name: setting,
                "aria-labelledby": `${id}-label`,
                "aria-describedby": id,
              },
            }}
            onChange={async (_, value) => {
              setSaving(true);
              try {
                await save({ setting, value });
              } finally {
                setSaving(false);
              }
            }}
          />
        }
      />
    </div>
  );
}

export function FetchNumber({
  setting,
  ...props
}: {
  setting: NumberSetting;
  label: string;
  description: string;
  unit: string;
  maximum?: bigint;
}) {
  const { settings, save } = useFetchSettings();
  return (
    <NumericSetting
      {...props}
      name={setting}
      conversion={
        setting === "feedMaxBodyBytes"
          ? {
              format: bytesToMib,
              parse: mibToBytes,
              error:
                "Enter MiB within the supported range, equal to a whole number of bytes.",
            }
          : undefined
      }
      savedValue={settings?.[setting] ?? null}
      saveValue={(value) => save({ setting, value })}
    />
  );
}
