import { useContext } from "react";
import EmbeddingStartupContext from "../providers/EmbeddingStartupContext";
import type { EmbeddingStartupState } from "../platform/Platform";

export function useEmbeddingStartupState(): EmbeddingStartupState {
  return useContext(EmbeddingStartupContext);
}
