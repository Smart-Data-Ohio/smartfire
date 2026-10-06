import { useNavigate } from "@tanstack/react-router";
import { useEffect, useEffectEvent } from "react";
import { IS_APPLE } from "../../lib/shortcuts.ts";
import { isEditable, searchShortcut } from "./search-keys.ts";

/** The search field on screen, if one is showing (the room header's, or the search page's). */
export function visibleSearchInput(): HTMLInputElement | null {
  const fields = document.querySelectorAll<HTMLInputElement>("input[data-search-input]");

  return [...fields].find((field) => field.getClientRects().length > 0) ?? null;
}

/**
 * ⌘⇧F (Ctrl+Shift+F) anywhere, or `/` outside a text field: focus the search field on screen,
 * or open the search page when none is (a phone, the home view). Mounted once by the shell; it
 * stays out of the way of an open dialog and of keys a focused control already handled.
 */
export function SearchHotkey() {
  const navigate = useNavigate();

  const onKeyDown = useEffectEvent((event: KeyboardEvent) => {
    if (event.defaultPrevented || event.isComposing) {
      return;
    }

    const shortcut = searchShortcut(event, IS_APPLE, isEditable(event.target));

    if (shortcut === null || document.querySelector("dialog[open]") !== null) {
      return;
    }

    event.preventDefault();

    const field = visibleSearchInput();

    if (field === null) {
      void navigate({ to: "/search" });

      return;
    }

    field.focus();
    field.select();
  });

  useEffect(() => {
    const listener = (event: KeyboardEvent) => onKeyDown(event);

    window.addEventListener("keydown", listener);

    return () => window.removeEventListener("keydown", listener);
  }, []);

  return null;
}
