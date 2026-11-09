import { FormHelperText, Stack, Typography } from "@mui/material";
import type { ReactNode } from "react";

export function SettingsControlLabel({
  id,
  label,
  children,
}: {
  id: string;
  label: string;
  children: ReactNode;
}) {
  return (
    <Stack component="span" sx={{ paddingTop: "0.5625rem" }}>
      <Typography component="span" id={`${id}-label`}>
        {label}
      </Typography>
      <FormHelperText component="span" id={id}>
        {children}
      </FormHelperText>
    </Stack>
  );
}
