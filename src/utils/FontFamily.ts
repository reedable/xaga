export type FontFamily = "system" | "serif" | "sans-serif" | "monospace";

/**
 * Maps an explicit reader category to lliwio's corresponding family token.
 * `system` leaves lliwio's primary family untouched, so lliwio remains the
 * authority for every category's concrete font stack.
 */
export const FONT_FAMILY_CSS: Record<FontFamily, string | null> = {
  system: null,
  serif: "var(--lliw-typography__font-family--serif)",
  "sans-serif": "var(--lliw-typography__font-family--sans-serif)",
  monospace: "var(--lliw-typography__font-family--monospace)",
};
