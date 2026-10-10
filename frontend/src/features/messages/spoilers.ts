import { useLayoutEffect, useRef } from "react";

/** What a screen reader hears while the spoiler is still covered. The words stay in the DOM. */
export const SPOILER_LABEL = "Spoiler, activate to reveal";

const SPOILER = "[data-spoiler], .spoiler";

/** Attributes that name an element or show a tooltip, stashed on the element until it is revealed. */
const CONCEALED = [
  ["title", "data-spoiler-title"],
  ["aria-label", "data-spoiler-label"],
  ["aria-labelledby", "data-spoiler-labelledby"],
  ["aria-describedby", "data-spoiler-describedby"],
  ["alt", "data-spoiler-alt"],
] as const;

/**
 * Cover each spoiler and reveal that one on click, Enter, or Space. The event does not bubble
 * into the message row. Returns the cleanup for the listeners.
 */
export function bindSpoilers(root: HTMLElement): () => void {
  prepareSpoilers(root);

  const onClick = (event: MouseEvent) => {
    const spoiler = spoilerElement(event.target);

    if (spoiler === null) {
      return;
    }

    // Enter/Space reveals on keydown, then the button's click follows. That click must not
    // reach the message row either. A later mouse click on an open spoiler still can.
    if (spoiler.hasAttribute("data-revealed")) {
      if (event.detail === 0) {
        event.preventDefault();
        event.stopPropagation();
      }

      return;
    }

    reveal(spoiler);
    event.preventDefault();
    event.stopPropagation();
  };

  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key !== "Enter" && event.key !== " ") {
      return;
    }

    const spoiler = hiddenSpoiler(event.target);

    if (spoiler === null) {
      return;
    }

    reveal(spoiler);
    event.preventDefault();
    event.stopPropagation();
  };

  const onPointerDown = (event: PointerEvent) => {
    if (hiddenSpoiler(event.target) !== null) {
      event.stopPropagation();
    }
  };

  root.addEventListener("click", onClick);
  root.addEventListener("keydown", onKeyDown);
  root.addEventListener("pointerdown", onPointerDown);

  return () => {
    root.removeEventListener("click", onClick);
    root.removeEventListener("keydown", onKeyDown);
    root.removeEventListener("pointerdown", onPointerDown);
  };
}

/** Ref for a node whose `innerHTML` is message HTML. Spoilers on it reveal the same way as a message. */
export function useSpoilerReveal(html: string) {
  const ref = useRef<HTMLDivElement>(null);

  // `html` is the markup React just wrote into the node. The ref object does not change with it,
  // so the effect would otherwise keep the previous spoilers' listeners on the new markup.
  // biome-ignore lint/correctness/useExhaustiveDependencies: html is that inserted markup
  useLayoutEffect(() => {
    const root = ref.current;

    if (root === null) {
      return;
    }

    return bindSpoilers(root);
  }, [html]);

  return ref;
}

/** Makes each still-hidden spoiler a button, and its contents neither focusable nor named. */
function prepareSpoilers(root: HTMLElement): void {
  for (const node of root.querySelectorAll(SPOILER)) {
    if (!(node instanceof HTMLElement) || node.hasAttribute("data-revealed")) {
      continue;
    }

    node.tabIndex = 0;
    node.setAttribute("role", "button");
    node.setAttribute("aria-label", SPOILER_LABEL);
    conceal(node);
  }
}

/** While covered, descendants are not in the tab order or the accessibility tree, and have no tooltip. */
function conceal(spoiler: HTMLElement): void {
  for (const node of spoiler.querySelectorAll<HTMLElement>("*")) {
    node.inert = true;

    if (node.tabIndex >= 0) {
      node.setAttribute("data-spoiler-tabindex", node.getAttribute("tabindex") ?? "");
      node.tabIndex = -1;
    }

    for (const [name, stash] of CONCEALED) {
      if (node.hasAttribute(name)) {
        node.setAttribute(stash, node.getAttribute(name) ?? "");
        node.removeAttribute(name);
      }
    }

    // `inert` keeps them out of a real accessibility tree. jsdom still names them from their
    // text, so hide them explicitly and put the previous value back on reveal.
    if (!node.hasAttribute("data-spoiler-hidden")) {
      node.setAttribute("data-spoiler-hidden", node.getAttribute("aria-hidden") ?? "");
      node.setAttribute("aria-hidden", "true");
    }
  }
}

/** The spoiler element `target` is in, covered or already revealed. */
function spoilerElement(target: EventTarget | null): HTMLElement | null {
  if (!(target instanceof Element)) {
    return null;
  }

  const spoiler = target.closest(SPOILER);

  return spoiler instanceof HTMLElement ? spoiler : null;
}

/** A spoiler that is still covered. */
function hiddenSpoiler(target: EventTarget | null): HTMLElement | null {
  const spoiler = spoilerElement(target);

  if (spoiler === null || spoiler.hasAttribute("data-revealed")) {
    return null;
  }

  return spoiler;
}

/** Shows this spoiler's text and gives its contents their names and focus back. */
function reveal(spoiler: HTMLElement): void {
  spoiler.setAttribute("data-revealed", "");
  spoiler.removeAttribute("aria-label");
  spoiler.removeAttribute("role");
  spoiler.removeAttribute("tabindex");

  for (const node of spoiler.querySelectorAll<HTMLElement>("*")) {
    node.inert = false;

    if (node.hasAttribute("data-spoiler-tabindex")) {
      const previous = node.getAttribute("data-spoiler-tabindex") ?? "";

      if (previous === "") {
        node.removeAttribute("tabindex");
      } else {
        node.setAttribute("tabindex", previous);
      }

      node.removeAttribute("data-spoiler-tabindex");
    }

    for (const [name, stash] of CONCEALED) {
      if (node.hasAttribute(stash)) {
        node.setAttribute(name, node.getAttribute(stash) ?? "");
        node.removeAttribute(stash);
      }
    }

    if (node.hasAttribute("data-spoiler-hidden")) {
      const previous = node.getAttribute("data-spoiler-hidden") ?? "";

      if (previous === "") {
        node.removeAttribute("aria-hidden");
      } else {
        node.setAttribute("aria-hidden", previous);
      }

      node.removeAttribute("data-spoiler-hidden");
    }
  }
}
