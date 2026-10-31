# Motion

Use a no-animation-first approach. Base styles present a complete, usable static
state. Add animation only inside `@media (prefers-reduced-motion: no-preference)`;
when the user requests reduced motion or the media feature is unsupported, the
static state remains.

```css
.ReloadIcon {
  animation: none;
}

@media (prefers-reduced-motion: no-preference) {
  @keyframes reload-spin {
    to { transform: rotate(360deg); }
  }

  .ReloadIconBusy {
    animation: reload-spin 1s linear infinite;
  }
}
```

Treat transitions and component-library motion the same way. Keep motion rules
in CSS where possible. If a component's built-in behavior prevents this pattern,
surface the limitation and its alternatives for the user's decision.

Animation must not be the only way to convey state. Provide appropriate static
text and screen-reader feedback independently of motion. For Refresh, the
in-progress button title provides nonanimated sighted feedback; SR-only progress
feedback is separate.
