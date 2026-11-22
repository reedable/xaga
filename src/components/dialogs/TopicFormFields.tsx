/* oxlint-disable react/only-export-components -- the form hook and fields are one shared unit */
import { Add, Remove } from "@mui/icons-material";
import { Box, IconButton, TextField, Tooltip, Typography } from "@mui/material";
import { useCallback, useState } from "react";
import ShortcutDestination, {
  useShortcutSelection,
} from "./ShortcutDestination";
import { TOPIC_THRESHOLD } from "../../consts";

export interface TopicFormValues {
  title: string;
  description: string;
  threshold: number;
}

interface TopicFormInitialValues {
  title?: string;
  description?: string;
  threshold?: number;
}

export function useTopicForm(initial: TopicFormInitialValues = {}) {
  const [title, setTitle] = useState(initial.title ?? "");
  const [description, setDescription] = useState(initial.description ?? "");
  const [thresholdString, setThresholdString] = useState(
    (initial.threshold ?? TOPIC_THRESHOLD).toFixed(2)
  );

  const parseThreshold = useCallback(() => {
    const value = parseFloat(thresholdString);
    if (Number.isNaN(value)) return TOPIC_THRESHOLD;
    return Math.max(-1, Math.min(1, value));
  }, [thresholdString]);

  const reset = useCallback((values: TopicFormInitialValues = {}) => {
    setTitle(values.title ?? "");
    setDescription(values.description ?? "");
    setThresholdString((values.threshold ?? TOPIC_THRESHOLD).toFixed(2));
  }, []);

  return {
    title,
    setTitle,
    description,
    setDescription,
    thresholdString,
    setThresholdString,
    parseThreshold,
    reset,
    values: {
      title: title.trim(),
      description,
      threshold: parseThreshold(),
    } satisfies TopicFormValues,
  };
}

type TopicForm = ReturnType<typeof useTopicForm>;

export function TopicFormFields({
  form,
  shortcut,
  busy,
  entitySaved = false,
  autoFocus = false,
}: {
  form: TopicForm;
  shortcut: ReturnType<typeof useShortcutSelection>;
  busy: boolean;
  entitySaved?: boolean;
  autoFocus?: boolean;
}) {
  const decrement = () =>
    form.setThresholdString(
      Math.max(
        -1,
        parseFloat((form.parseThreshold() - 0.01).toFixed(2))
      ).toFixed(2)
    );
  const increment = () =>
    form.setThresholdString(
      Math.min(
        1,
        parseFloat((form.parseThreshold() + 0.01).toFixed(2))
      ).toFixed(2)
    );

  return (
    <>
      <TextField
        name="topic-title"
        label="Title"
        value={form.title}
        onChange={(event) => form.setTitle(event.target.value)}
        required
        autoFocus={autoFocus}
        disabled={busy || entitySaved}
        fullWidth
      />
      <TextField
        data-testid="topic-description"
        label="Description"
        value={form.description}
        onChange={(event) => form.setDescription(event.target.value)}
        multiline
        minRows={4}
        disabled={busy || entitySaved}
        fullWidth
      />
      <ShortcutDestination
        value={shortcut.value}
        onChange={shortcut.change}
        disabled={busy}
      />
      <Box>
        <Typography
          data-testid="topic-similarity"
          variant="caption"
          color="text.secondary"
          sx={{ display: "block" }}
          gutterBottom
        >
          Similarity threshold
        </Typography>
        <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
          <Tooltip title="Decrease threshold">
            <IconButton
              size="small"
              onClick={decrement}
              disabled={busy || entitySaved}
              aria-label="Decrease threshold"
            >
              <Remove fontSize="small" />
            </IconButton>
          </Tooltip>
          <TextField
            value={form.thresholdString}
            onChange={(event) => form.setThresholdString(event.target.value)}
            onBlur={() =>
              form.setThresholdString(form.parseThreshold().toFixed(2))
            }
            size="small"
            disabled={busy || entitySaved}
            sx={{ width: 80 }}
            slotProps={{ htmlInput: { style: { textAlign: "center" } } }}
          />
          <Tooltip title="Increase threshold">
            <IconButton
              size="small"
              onClick={increment}
              disabled={busy || entitySaved}
              aria-label="Increase threshold"
            >
              <Add fontSize="small" />
            </IconButton>
          </Tooltip>
        </Box>
      </Box>
    </>
  );
}
