// ── Annotation colour palette ─────────────────────────────────────────────────
//
// Each entry has exactly two tokens:
//   bg     — translucent fill applied to the <mark> background
//   accent — used for the mark outline, mark hover fill, and the popup left border
//
// Two separate maps keep light and dark values from bleeding into each other.
// Add new colours to both maps; every consumer derives its values from here.

export interface AnnotationColorTokens {
  bg: string;
  accent: string;
}

export const ANNOTATION_COLORS_LIGHT: Record<string, AnnotationColorTokens> = {
  none: {
    bg: "transparent",
    accent: "transparent",
    // accent: "rgba(100,100,100,1)",
  },
  red: {
    bg: "rgba(255,80,80,0.35)",
    accent: "rgba(210,40,40,0.60)",  // 0.65→0.60: hover bg L=0.330 → 4.91:1 with #212121
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
    accent: "rgba(120,30,210,0.50)",  // 0.65→0.50: hover bg L=0.362 → 5.32:1 with #212121
  },
};

// Dark mode: bright opaque fills so marks are visible on #121212.
// bg values are chosen so the blended color has L≥0.175 (≥4.5:1 with #000000 text).
// accent values are used for outline + hover bg; also ≥4.5:1 with #000000.
export const ANNOTATION_COLORS_DARK: Record<string, AnnotationColorTokens> = {
  none: {
    bg: "transparent",
    accent: "transparent",
  },
  red: {
    bg: "rgba(255,80,80,0.80)",      // eff rgb(208,68,68)  L=0.179 → 4.57:1
    accent: "rgba(255,130,130,0.75)", // eff rgb(196,102,102) L=0.222 → 5.44:1
  },
  orange: {
    bg: "rgba(255,160,40,0.65)",     // eff rgb(172,110,32)  L=0.201 → 5.02:1
    accent: "rgba(255,190,80,0.75)", // eff rgb(196,147,65)  L=0.330 → 7.60:1
  },
  yellow: {
    bg: "rgba(255,220,40,0.55)",     // eff rgb(148,129,30)  L=0.222 → 5.43:1
    accent: "rgba(255,230,80,0.75)", // eff rgb(196,177,65)  L=0.436 → 9.71:1
  },
  green: {
    bg: "rgba(60,190,60,0.70)",      // eff rgb(47,138,47)   L=0.191 → 4.82:1
    accent: "rgba(110,230,110,0.75)",// eff rgb(87,177,87)   L=0.342 → 7.83:1
  },
  blue: {
    bg: "rgba(60,160,255,0.75)",     // eff rgb(50,125,196)  L=0.192 → 4.84:1
    accent: "rgba(120,180,255,0.75)",// eff rgb(95,140,196)  L=0.253 → 6.05:1
  },
  violet: {
    bg: "rgba(200,100,255,0.80)",    // eff rgb(164,84,208)  L=0.187 → 4.73:1
    accent: "rgba(210,140,255,0.75)",// eff rgb(162,110,196) L=0.228 → 5.57:1
  },
};

/** Returns the tokens for a stored colour name, falling back to yellow. */
export function getAnnotationColor(
  color: string,
  dark = false,
): AnnotationColorTokens {
  const map = dark ? ANNOTATION_COLORS_DARK : ANNOTATION_COLORS_LIGHT;
  return map[color] ?? map.yellow;
}
