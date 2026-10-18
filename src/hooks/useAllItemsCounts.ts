import { useContext } from "react";
import AllItemsCountsContext from "../providers/AllItemsCountsContext";

export function useAllItemsCounts() {
  return useContext(AllItemsCountsContext);
}
