import { useCallback, useState } from "react";
import css from "./AddressBar.module.css";

interface AddressBarProps {
  url: string | null;
  onNavigate: (url: string) => void;
}

export function AddressBar({ url, onNavigate }: AddressBarProps) {
  // Non-null only while the user is actively editing.
  // Derived display value: user's input while editing, canonical url otherwise.
  const [editValue, setEditValue] = useState<string | null>(null);
  const displayValue = editValue ?? url ?? "";

  const handleFocus = useCallback(
    (e: React.FocusEvent<HTMLInputElement>) => {
      setEditValue(url ?? "");
      const target = e.target;
      requestAnimationFrame(() => target.select());
    },
    [url]
  );

  const handleBlur = useCallback(() => setEditValue(null), []);

  const handleChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setEditValue(e.target.value);
  }, []);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "Enter") {
        const trimmed = displayValue.trim();
        if (trimmed) onNavigate(trimmed);
        e.currentTarget.blur();
      } else if (e.key === "Escape") {
        e.currentTarget.blur();
      }
    },
    [displayValue, onNavigate]
  );

  return (
    <input
      className={css.AddressBarInput}
      value={displayValue}
      placeholder="Enter URL…"
      onChange={handleChange}
      onFocus={handleFocus}
      onBlur={handleBlur}
      onKeyDown={handleKeyDown}
      autoComplete="off"
      autoCorrect="off"
      autoCapitalize="off"
      spellCheck={false}
    />
  );
}
