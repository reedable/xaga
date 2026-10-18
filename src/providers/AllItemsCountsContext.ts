import { createContext } from "react";
import type { AllItemsCounts } from "../types";

const AllItemsCountsContext = createContext<AllItemsCounts | null>(null);

export default AllItemsCountsContext;
