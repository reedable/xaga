// The shared layout properties are rem lengths. Read their authoritative CSS
// values; callers convert browser geometry with the current root font size.
export function getLayoutWidth(element: Element, property: string): number {
  return parseFloat(
    getComputedStyle(element).getPropertyValue(property) ||
      getComputedStyle(document.documentElement).getPropertyValue(property)
  );
}
