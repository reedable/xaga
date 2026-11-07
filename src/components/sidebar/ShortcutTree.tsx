import { ChevronRight, MoreVert } from "@mui/icons-material";
import {
  Button,
  Collapse,
  useMediaQuery,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  IconButton,
  Tooltip,
  ListItemButton,
  Menu,
  MenuItem,
  TextField,
} from "@mui/material";
import {
  useEffect,
  useId,
  useState,
  type DragEvent,
  type ReactNode,
} from "react";
import { pinKey, type PinEdit, type PinNodeRef } from "../../pins";
import { usePins } from "../../providers/PinsContext";
import { usePlatform } from "../../platform/Platform";
import { useLibraryReadOnly } from "../../hooks/useLibraryReadOnly";
import { useErrorSink } from "../../hooks/useErrorSink";
import { getViewState, setViewState } from "../../storage/viewState";
import { filterPinTree, type ShortcutNode } from "../../utils/projectPinTree";
import UnreadCount from "./UnreadCount";
import css from "./ShortcutTree.module.css";

const DRAG_TYPE = "application/x-xaga-pin";

type Props = {
  nodes: ShortcutNode[];
  filter: string;
  createRequest: number;
  renderTarget: (node: ShortcutNode, unreadCount: ReactNode) => ReactNode;
};

export default function ShortcutTree({
  nodes,
  filter,
  createRequest,
  renderTarget,
}: Props) {
  const { organizePins, setPin } = usePins();
  const platform = usePlatform();
  const readOnly = useLibraryReadOnly();
  const animate = useMediaQuery("(prefers-reduced-motion: no-preference)");
  const { pushError } = useErrorSink();
  const [storageKey, setStorageKey] = useState<string>();
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});
  const [dragging, setDragging] = useState(false);
  const [busy, setBusy] = useState(false);

  const [titleDialog, setTitleDialog] = useState<{
    id?: string;
    parent: string | null;
    title: string;
  }>();

  const [moveDialog, setMoveDialog] = useState<ShortcutNode>();
  const [destination, setDestination] = useState("");
  const actionMenuId = useId();

  const [actionMenu, setActionMenu] = useState<{
    anchor: HTMLElement;
    node: ShortcutNode;
  }>();

  useEffect(() => {
    if (createRequest) setTitleDialog({ parent: null, title: "" });
  }, [createRequest]);

  useEffect(() => {
    let active = true;
    void (async () => {
      const key = `pins.groups.${await platform.getLibrary()}`;
      const saved = await getViewState<Record<string, boolean>>(key);
      if (active) {
        setExpanded((current) => ({ ...saved, ...current }));
        setStorageKey(key);
      }
    })().catch(pushError);
    return () => {
      active = false;
    };
  }, [platform, pushError]);

  function expand(id: string, value: boolean) {
    setExpanded((current) => ({ ...current, [id]: value }));
  }

  useEffect(() => {
    if (storageKey) void setViewState(storageKey, expanded).catch(pushError);
  }, [storageKey, expanded, pushError]);

  async function execute(operation: () => Promise<void>): Promise<boolean> {
    if (busy || readOnly) return false;
    setBusy(true);

    try {
      await operation();
      return true;
    } catch (error) {
      pushError(error);
      return false;
    } finally {
      setBusy(false);
    }
  }

  async function edit(edit: PinEdit) {
    return execute(() => organizePins(edit));
  }

  async function move(node: PinNodeRef, parent: string | null) {
    if (await edit({ action: "move", node, parent })) {
      if (parent) expand(parent, true);
      setMoveDialog(undefined);
    }
  }

  function drop(event: DragEvent, parent: string | null) {
    if (!event.dataTransfer.types.includes(DRAG_TYPE)) return;
    event.preventDefault();
    event.stopPropagation();
    setDragging(false);
    if (readOnly || busy) return;

    try {
      void move(
        JSON.parse(event.dataTransfer.getData(DRAG_TYPE)) as PinNodeRef,
        parent
      );
    } catch (error) {
      pushError(error);
    }
  }

  function allowDrop(event: DragEvent) {
    if (!readOnly && !busy && event.dataTransfer.types.includes(DRAG_TYPE)) {
      event.preventDefault();
      event.stopPropagation();
      event.dataTransfer.dropEffect = "move";
    }
  }

  function actions(node: ShortcutNode) {
    const open =
      !!actionMenu &&
      pinKey(actionMenu.node.reference) === pinKey(node.reference);
    const label = node.reference.type === "group" ? "group" : "shortcut";

    return (
      <span className={css.NodeActions}>
        <Tooltip
          title={
            node.reference.type === "group"
              ? "Group actions"
              : "Shortcut actions"
          }
        >
          <span>
            <IconButton
              className={css.NodeActionButton}
              size="small"
              aria-label={`Actions for ${label} ${node.title}`}
              aria-haspopup="menu"
              aria-expanded={open}
              aria-controls={open ? actionMenuId : undefined}
              disabled={readOnly || busy}
              onClick={(event) =>
                setActionMenu({ anchor: event.currentTarget, node })
              }
            >
              <MoreVert fontSize="small" />
            </IconButton>
          </span>
        </Tooltip>
      </span>
    );
  }

  function render(nodes: ShortcutNode[]): ReactNode {
    return nodes.map((node) => {
      const group =
        node.reference.type === "group" ? node.reference : undefined;
      const isOpen = !!filter.trim() || (group && expanded[group.id] !== false);

      return (
        <div
          key={pinKey(node.reference)}
          className={css.TreeNode}
          draggable={!readOnly && !busy}
          onDragStart={(event) => {
            setDragging(true);
            event.stopPropagation();
            event.dataTransfer.setData(
              DRAG_TYPE,
              JSON.stringify(node.reference)
            );
            event.dataTransfer.effectAllowed = "move";
          }}
          onDragEnd={() => setDragging(false)}
          onDragOver={group ? allowDrop : undefined}
          onDrop={group ? (event) => drop(event, group.id) : undefined}
        >
          <div
            className={css.NodeRow}
            data-actions-open={
              (actionMenu &&
                pinKey(actionMenu.node.reference) === pinKey(node.reference)) ||
              undefined
            }
          >
            {group ? (
              <ListItemButton
                onClick={() => {
                  if (!filter.trim()) expand(group.id, !isOpen);
                }}
                aria-disabled={!!filter.trim()}
                aria-expanded={!!isOpen}
              >
                <ChevronRight
                  sx={{
                    transform: isOpen ? "rotate(90deg)" : undefined,
                    fontSize: 16,
                  }}
                />
                <span className={css.GroupTitle}>{node.title}</span>
                <span className={css.NodeUnreadCount}>
                  <UnreadCount count={node.unreadCount} />
                </span>
              </ListItemButton>
            ) : (
              <div className={css.TargetRow}>
                {renderTarget(
                  node,
                  <span className={css.NodeUnreadCount}>
                    <UnreadCount count={node.unreadCount} />
                  </span>
                )}
              </div>
            )}
            {actions(node)}
          </div>

          {group && (
            <Collapse
              in={!!isOpen}
              unmountOnExit={false}
              timeout={animate ? "auto" : 0}
            >
              <div className={css.GroupChildrenContainer}>
                <div className={css.GroupChildren}>
                  {node.children?.length ? (
                    render(node.children)
                  ) : (
                    <span className={css.Empty}>Empty</span>
                  )}
                </div>
              </div>
            </Collapse>
          )}
        </div>
      );
    });
  }

  function destinations(
    nodes: ShortcutNode[],
    prefix = ""
  ): { id: string; title: string }[] {
    return nodes.flatMap((node) => {
      if (
        node.reference.type !== "group" ||
        (moveDialog && pinKey(node.reference) === pinKey(moveDialog.reference))
      )
        return [];
      const title = `${prefix}${node.title}`;
      return [
        { id: node.reference.id, title },
        ...destinations(node.children ?? [], `${title} / `),
      ];
    });
  }

  return (
    <>
      <div
        className={css.RootDrop}
        onDragOver={allowDrop}
        onDrop={(event) => drop(event, null)}
      >
        {render(filterPinTree(nodes, filter))}
        {dragging && (
          <div className={css.RootDestination}>Move to top level of Shortcuts</div>
        )}
      </div>

      <Menu
        id={actionMenuId}
        anchorEl={actionMenu?.anchor}
        open={!!actionMenu}
        onClose={() => setActionMenu(undefined)}
      >
        {actionMenu?.node.reference.type === "group" && [
          <MenuItem
            key="add"
            disabled={readOnly || busy}
            onClick={() => {
              if (actionMenu?.node.reference.type !== "group") return;
              setTitleDialog({
                parent: actionMenu.node.reference.id,
                title: "",
              });
              setActionMenu(undefined);
            }}
          >
            Add group
          </MenuItem>,
          <MenuItem
            key="rename"
            disabled={readOnly || busy}
            onClick={() => {
              if (actionMenu?.node.reference.type !== "group") return;
              setTitleDialog({
                id: actionMenu.node.reference.id,
                parent: null,
                title: actionMenu.node.title,
              });
              setActionMenu(undefined);
            }}
          >
            Rename group
          </MenuItem>,
        ]}
        <MenuItem
          disabled={readOnly || busy}
          onClick={() => {
            if (!actionMenu) return;
            setMoveDialog(actionMenu.node);
            setDestination("");
            setActionMenu(undefined);
          }}
        >
          Move to…
        </MenuItem>
        <MenuItem
          disabled={readOnly || busy}
          onClick={() => {
            if (!actionMenu) return;
            const reference = actionMenu.node.reference;
            if (reference.type === "group")
              void edit({ action: "removeGroup", id: reference.id });
            else void execute(() => setPin(reference, false));
            setActionMenu(undefined);
          }}
        >
          {actionMenu?.node.reference.type === "group"
            ? "Remove group"
            : "Remove shortcut"}
        </MenuItem>
      </Menu>

      <Dialog
        open={!!titleDialog}
        onClose={() => !busy && setTitleDialog(undefined)}
      >
        <form
          onSubmit={(event) => {
            event.preventDefault();
            if (!titleDialog?.title.trim()) return;
            const dialog = titleDialog;
            void edit(
              dialog.id
                ? {
                    action: "renameGroup",
                    id: dialog.id,
                    title: dialog.title.trim(),
                  }
                : {
                    action: "createGroup",
                    id: crypto.randomUUID(),
                    title: dialog.title.trim(),
                    parent: dialog.parent,
                  }
            ).then((saved) => {
              if (saved) setTitleDialog(undefined);
            });
          }}
        >
          <DialogTitle>
            {titleDialog?.id ? "Rename group" : "Create group"}
          </DialogTitle>
          <DialogContent>
            <TextField
              autoFocus
              label="Title"
              margin="normal"
              value={titleDialog?.title ?? ""}
              disabled={busy}
              onChange={(event) =>
                setTitleDialog(
                  (current) =>
                    current && { ...current, title: event.target.value }
                )
              }
            />
          </DialogContent>
          <DialogActions>
            <Button onClick={() => setTitleDialog(undefined)} disabled={busy}>
              Cancel
            </Button>
            <Button
              type="submit"
              disabled={readOnly || busy || !titleDialog?.title.trim()}
            >
              {titleDialog?.id ? "Save" : "Create group"}
            </Button>
          </DialogActions>
        </form>
      </Dialog>

      <Dialog
        open={!!moveDialog}
        onClose={() => !busy && setMoveDialog(undefined)}
      >
        <DialogTitle>Move {moveDialog?.title} to…</DialogTitle>
        <DialogContent>
          <TextField
            select
            label="Destination"
            margin="normal"
            slotProps={{ select: { displayEmpty: true }, inputLabel: { shrink: true } }}
            value={destination}
            fullWidth
            disabled={busy}
            onChange={(event) => setDestination(event.target.value)}
          >
            <MenuItem value="">Top level of Shortcuts</MenuItem>
            {destinations(nodes).map((group) => (
              <MenuItem key={group.id} value={group.id}>
                {group.title}
              </MenuItem>
            ))}
          </TextField>
        </DialogContent>

        <DialogActions>
          <Button disabled={busy} onClick={() => setMoveDialog(undefined)}>
            Cancel
          </Button>
          <Button
            disabled={readOnly || busy}
            onClick={() =>
              moveDialog && void move(moveDialog.reference, destination || null)
            }
          >
            Move
          </Button>
        </DialogActions>
      </Dialog>
    </>
  );
}
