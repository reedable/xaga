import { Drawer } from "@mui/material";
import {
  useCallback,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import css from "./SearchPanel.module.css";

// Move one portal host between presentations so input, result and scroll state
// survive a breakpoint change without remounting SearchResultList.
export function SearchPanel({
  drawer,
  onClose,
  children,
}: {
  drawer: boolean;
  onClose: () => void;
  children: ReactNode;
}) {
  const [panel] = useState(() => {
    const element = document.createElement("div");
    element.className = css.Panel;
    return element;
  });
  const [opener] = useState(() => document.activeElement as HTMLElement | null);
  const inlineHost = useRef<HTMLDivElement>(null);
  const drawerHost = useRef<HTMLDivElement>(null);
  const initialized = useRef(false);
  const movePanel = useCallback(
    (host: HTMLDivElement | null) => {
      if (!host) return;
      const focused = document.activeElement as HTMLElement | null;
      const retainedFocus = focused && panel.contains(focused) ? focused : null;
      host.append(panel);
      if (retainedFocus) retainedFocus.focus({ preventScroll: true });
      else if (drawer || !initialized.current)
        panel.querySelector("input")?.focus({ preventScroll: true });
      initialized.current = true;
    },
    [drawer, panel]
  );
  const attachDrawer = useCallback(
    (host: HTMLDivElement | null) => {
      drawerHost.current = host;
      if (drawer) movePanel(host);
    },
    [drawer, movePanel]
  );
  useLayoutEffect(() => {
    movePanel(drawer ? drawerHost.current : inlineHost.current);
  }, [drawer, movePanel]);
  useLayoutEffect(
    () => () => {
      panel.remove();
      requestAnimationFrame(() => {
        if (opener?.isConnected) opener.focus({ preventScroll: true });
      });
    },
    [panel, opener]
  );
  return (
    <>
      <div
        ref={inlineHost}
        className={drawer ? css.Hidden : css.Inline}
        data-testid="search-inline"
      />
      <Drawer
        anchor="right"
        open={drawer}
        onClose={onClose}
        transitionDuration={0}
        ModalProps={{
          keepMounted: true,
          disableAutoFocus: true,
          disableRestoreFocus: true,
        }}
        slotProps={{
          paper: {
            role: "dialog",
            "aria-label": "Search",
            "aria-modal": true,
            className: css.Drawer,
          },
        }}
      >
        <div ref={attachDrawer} className={css.Panel} />
      </Drawer>
      {createPortal(children, panel)}
    </>
  );
}
