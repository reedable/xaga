import { MemoryRouter as Router } from "react-router-dom";
import type { ComponentProps } from "react";
import { NavigationProvider } from "../providers/NavigationProvider";

export function MemoryRouter({
  children,
  ...props
}: ComponentProps<typeof Router>) {
  return (
    <Router {...props}>
      <NavigationProvider>{children}</NavigationProvider>
    </Router>
  );
}
