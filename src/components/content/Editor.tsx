import { markdown } from "@codemirror/lang-markdown";
import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import EditIcon from "@mui/icons-material/Edit";
import VisibilityIcon from "@mui/icons-material/Visibility";
import { Box, IconButton, Tooltip } from "@mui/material";
import { useEffect, useRef, useState } from "react";
import { markdownToHtml } from "../../text/markdownPipeline";
import type { Note } from "../../types";
import cx from "../../utils/cx";
import { Reader } from "./Reader";

interface EditorProps {
  className?: string;
  note: Note;
}

export function Editor({ className, note }: EditorProps) {
  const [editMode, setEditMode] = useState(false);
  const [html, setHtml] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);

  useEffect(() => {
    if (editMode) return;
    markdownToHtml(note.content).then(setHtml);
  }, [note.content, editMode]);

  useEffect(() => {
    if (!editMode || !containerRef.current) return;

    const view = new EditorView({
      state: EditorState.create({
        doc: note.content,
        extensions: [
          markdown(),
          EditorView.lineWrapping,
          EditorView.theme({
            "&": { height: "100%", fontFamily: "inherit", fontSize: "14px" },
            ".cm-scroller": { overflow: "auto" },
            ".cm-content": {
              padding: "24px 32px",
              maxWidth: "72ch",
              boxSizing: "border-box",
            },
            ".cm-line": { padding: "0" },
          }),
        ],
      }),
      parent: containerRef.current,
    });
    viewRef.current = view;

    return () => {
      view.destroy();
      viewRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editMode]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: note.content },
    });
  }, [note.content]);

  return (
    <Box
      className={cx(className)}
      sx={{ height: "100%", position: "relative" }}
    >
      <Tooltip
        title={editMode ? "Switch to reader" : "Switch to editor"}
        placement="left"
      >
        <IconButton
          size="small"
          onClick={() => setEditMode((m) => !m)}
          sx={{ position: "absolute", top: 8, right: 8, zIndex: 1 }}
        >
          {editMode ? (
            <VisibilityIcon fontSize="small" />
          ) : (
            <EditIcon fontSize="small" />
          )}
        </IconButton>
      </Tooltip>

      {editMode ? (
        <Box
          ref={containerRef}
          sx={{
            height: "100%",
            "& .cm-editor": { height: "100%" },
            "& .cm-focused": { outline: "none" },
          }}
        />
      ) : (
        html !== null && <Reader html={html} />
      )}
    </Box>
  );
}
