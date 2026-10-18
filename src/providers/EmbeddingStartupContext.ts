import { createContext } from "react";
import type { EmbeddingStartupState } from "../platform/Platform";

const EmbeddingStartupContext = createContext<EmbeddingStartupState>("loading");

export default EmbeddingStartupContext;
