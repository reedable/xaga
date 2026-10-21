import { createContext } from "react";
import type { Site } from "../types";

export type SitesContextType = {
  sites: Site[];
  refreshSites: () => Promise<void>;
};

const SitesContext = createContext<SitesContextType | null>(null);

export default SitesContext;
