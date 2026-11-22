import { useLayoutEffect, useRef, useState } from "react";
import type { TopicFormValues } from "../components/dialogs/TopicFormFields";
import { usePlatform } from "../platform/Platform";
import { errorMessage } from "../platform/errors";
import { isAbortError } from "../types";
import type { Topic } from "../types";

type Warning = {
  draft: TopicFormValues;
  matches: Topic[] | null;
};

export function useTopicCreationCheck(
  enabled: boolean,
  save: (draft: TopicFormValues) => Promise<void>
) {
  const platform = usePlatform();
  const request = useRef<AbortController | null>(null);
  const pendingWarning = useRef<Warning | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [warning, setWarning] = useState<Warning | null>(null);
  const [phase, setPhase] = useState<"checking" | "saving" | null>(null);

  useLayoutEffect(() => {
    setPhase(null);
    setWarning(null);
    setSaveError(null);
    return () => {
      request.current?.abort();
      request.current = null;
      pendingWarning.current = null;
    };
  }, [enabled, platform]);

  function dismiss() {
    pendingWarning.current = null;
    setWarning(null);
  }

  async function run(draft: TopicFormValues, confirmed: boolean) {
    if (!enabled || request.current) return;
    const controller = new AbortController();
    request.current = controller;
    setSaveError(null);
    setPhase(confirmed ? "saving" : "checking");

    try {
      if (!confirmed) {
        let matches: Topic[] | null;
        try {
          matches = await platform.searchSimilarTopics(
            controller.signal,
            draft.title,
            draft.description
          );
        } catch {
          matches = null;
        }
        if (controller.signal.aborted) return;
        if (matches === null || matches.length > 0) {
          const next = { draft, matches };
          pendingWarning.current = next;
          setWarning(next);
          return;
        }
      }
      setPhase("saving");
      await save(draft);
    } catch (error) {
      if (!controller.signal.aborted && !isAbortError(error)) {
        setSaveError(errorMessage(error));
      }
    } finally {
      if (request.current === controller) {
        request.current = null;
        setPhase(null);
      }
    }
  }

  function submit(draft: TopicFormValues, alreadySaved = false) {
    if (!pendingWarning.current) void run(draft, alreadySaved);
  }

  function confirm() {
    const pending = pendingWarning.current;
    if (!pending || !enabled || request.current) return;
    dismiss();
    void run(pending.draft, true);
  }

  return {
    warning,
    saveError,
    phase,
    busy: phase !== null,
    submit,
    confirm,
    dismiss,
  };
}
