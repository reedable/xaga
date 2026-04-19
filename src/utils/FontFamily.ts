export type FontFamily = "system" | "serif" | "sans-serif" | "monospace";

export const FONT_FAMILY_CSS: Record<FontFamily, string> = {
  system: 'Roboto,"Helvetica Neue",Arial,sans-serif',
  serif: 'Georgia,"Times New Roman",serif',
  "sans-serif": '"Helvetica Neue",Helvetica,Arial,sans-serif',
  monospace: '"Courier New",Courier,monospace',
};
