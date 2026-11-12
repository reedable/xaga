import { Drawer, useMediaQuery } from "@mui/material";
import { useEffect, useRef, type ReactNode } from "react";
import css from "./SearchPanel.module.css";

export function SearchPanel({
  drawer,
  open,
  onClose,
  children,
}: {
  drawer: boolean;
  open: boolean;
  onClose: () => void;
  children: ReactNode;
}) {
  const animate = useMediaQuery("(prefers-reduced-motion: no-preference)");
  const inlineHost = useRef<HTMLDivElement>(null);
  const drawerHost = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (open)
      (drawer ? drawerHost : inlineHost).current
        ?.querySelector("input")
        ?.focus({ preventScroll: true });
  }, [open, drawer]);
  return (
    <>
      <div
        ref={inlineHost}
        className={drawer ? css.Hidden : css.Inline}
        data-testid="search-inline"
      >
        {children}
      </div>
      <Drawer
        anchor="right"
        open={drawer && open}
        onClose={onClose}
        transitionDuration={animate ? 300 : 0}
        ModalProps={{ keepMounted: true }}
        slotProps={{
          paper: {
            role: "dialog",
            "aria-label": "Search",
            "aria-modal": true,
            className: css.Drawer,
          },
        }}
      >
        <div ref={drawerHost} className={css.Panel}>
          {children}
        </div>
      </Drawer>
    </>
  );
}
