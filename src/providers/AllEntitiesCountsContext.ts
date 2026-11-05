import { createContext } from "react";
import type { AllEntitiesCounts } from "../types";

const AllEntitiesCountsContext = createContext<AllEntitiesCounts | null>(null);

export default AllEntitiesCountsContext;
