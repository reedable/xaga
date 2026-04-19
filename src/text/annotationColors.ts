// ── Annotation colour palette ─────────────────────────────────────────────────
//
// Each entry has exactly two tokens:
//   bg     — translucent fill applied to the <mark> background
//   accent — used for the mark outline, mark hover fill, and the popup left border
//
// Add new colours here; every consumer derives its values from this record.

export interface AnnotationColorTokens {
  bg: string;
  accent: string;
}

export const ANNOTATION_COLORS: Record<string, AnnotationColorTokens> = {
  none: {
    bg: "transparent",
    accent: "transparent",
    // accent: "rgba(100,100,100,1)",
  },
  red: {
    bg: "rgba(255,80,80,0.35)",
    accent: "rgba(210,40,40,0.65)",
  },
  orange: {
    bg: "rgba(255,150,0,0.40)",
    accent: "rgba(210,120,0,0.65)",
  },
  yellow: {
    bg: "rgba(255,214,0,0.40)",
    accent: "rgba(255,180,0,0.60)",
  },
  green: {
    bg: "rgba(60,190,60,0.35)",
    accent: "rgba(30,150,30,0.65)",
  },
  blue: {
    bg: "rgba(60,130,255,0.35)",
    accent: "rgba(30,90,220,0.65)",
  },
  violet: {
    bg: "rgba(160,70,255,0.35)",
    accent: "rgba(120,30,210,0.65)",
  },
};

/** Returns the tokens for a stored colour name, falling back to yellow. */
export function getAnnotationColor(color: string): AnnotationColorTokens {
  return ANNOTATION_COLORS[color] ?? ANNOTATION_COLORS.yellow;
}
