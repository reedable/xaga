import { useContext } from "react";
import AllEntitiesCountsContext from "../providers/AllEntitiesCountsContext";

export function useAllEntitiesCounts() {
  return useContext(AllEntitiesCountsContext);
}
