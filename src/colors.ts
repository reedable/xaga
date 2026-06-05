export const COLOR_NAMES = [
  "red",
  "orange",
  "yellow",
  "green",
  "blue",
  "violet",
] as const;
export type ColorName = (typeof COLOR_NAMES)[number];
