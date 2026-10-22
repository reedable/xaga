import { useCallback, useState } from "react";

export function useSidebarFilter(library: string) {
  const [state, setState] = useState({ library, filter: "" });
  const filter = state.library === library ? state.filter : "";

  const setFilter = useCallback(
    (value: string) => setState({ library, filter: value }),
    [library]
  );

  return [filter, setFilter] as const;
}
