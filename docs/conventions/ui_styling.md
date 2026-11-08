# UI styling

Use `rem` rather than `px` for UI lengths, including spacing, borders, and
screen-reader-only dimensions. Use multiples of `0.0625rem` (one sixteenth of a
rem), written in decimal notation: for example, `0.0625rem`, `0.125rem`,
`0.25rem`, and `1.25rem`. Write negative values as decimals too, such as
`-0.0625rem`. Zero may be unitless.

Keep dimensions in `rem` in documentation and examples. Browser computed styles
resolve lengths to pixels; tests should normalize these measurements by the root
font size and verify scaling when the root font size changes.
