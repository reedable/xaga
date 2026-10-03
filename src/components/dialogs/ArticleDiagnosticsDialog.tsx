import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Typography,
} from "@mui/material";
import { useCallback, useEffect, useState } from "react";
import { usePlatform } from "../../platform/Platform";
import type { ArticleDiagnostic } from "../../types";

type ArticleDiagnosticsDialogProps = {
  open: boolean;
  diagnosticId: string;
  onClose: () => void;
};

export default function ArticleDiagnosticsDialog({
  open,
  diagnosticId,
  onClose,
}: ArticleDiagnosticsDialogProps) {
  const platform = usePlatform();
  const [diagnostic, setDiagnostic] = useState<ArticleDiagnostic | null>(null);
  const [status, setStatus] = useState("");

  useEffect(() => {
    if (!open) return;
    let active = true;

    platform.getArticleDiagnostic(diagnosticId).then(
      (result) => {
        if (active) setDiagnostic(result);
      },
      () => {
        if (active) setStatus("Diagnostics could not be read.");
      }
    );

    return () => {
      active = false;
    };
  }, [diagnosticId, open, platform]);

  const handleCopy = useCallback(async () => {
    if (!diagnostic) return;
    try {
      await navigator.clipboard.writeText(JSON.stringify(diagnostic, null, 2));
      setStatus("Diagnostics copied.");
    } catch {
      setStatus(
        "Clipboard access failed. Diagnostics are also in the Xaga log."
      );
    }
  }, [diagnostic]);

  return (
    <Dialog open={open} onClose={onClose} maxWidth="sm" fullWidth>
      <DialogTitle>Article loading diagnostics</DialogTitle>
      <DialogContent>
        <Typography variant="body2">
          {diagnostic
            ? `Attempt ${diagnostic.id}`
            : "No recent diagnostic record available."}
        </Typography>
        {diagnostic?.events
          .filter(
            (event) => event.outcome === "failed" || event.outcome === "warning"
          )
          .map((event, index) => (
            <Typography key={index} variant="body2" sx={{ mt: 1 }}>
              {event.step.replaceAll("_", " ")}: {event.code || event.outcome} (
              {event.elapsedMs} ms)
            </Typography>
          ))}
        <Typography variant="body2" sx={{ mt: 2 }}>
          {diagnostic?.events.some((event) => event.download) &&
            "Download diagnostics include URLs, query parameters, and local paths. Review before sharing."}
        </Typography>
        <Typography variant="body2" sx={{ mt: 2 }}>
          {status}
        </Typography>
      </DialogContent>
      <DialogActions>
        <Button disabled={!diagnostic} onClick={handleCopy}>
          Copy diagnostics
        </Button>
        <Button onClick={onClose}>Close</Button>
      </DialogActions>
    </Dialog>
  );
}
