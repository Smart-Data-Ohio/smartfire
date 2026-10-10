import { type RefCallback, useCallback } from "react";

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
 * Cover each spoiler and reveal that one on click, Enter, or Space. Only a covered spoiler takes
 * the event, and it does not bubble into the message row. Once revealed, its links and the row
 * get clicks and keys as usual. Returns the cleanup for the listeners.
 */
export function bindSpoilers(root: HTMLElement): () => void {
  prepareSpoilers(root);

  const onClick = (event: MouseEvent) => {
    const spoiler = hiddenSpoiler(event.target);

    if (spoiler === null) {
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

/**
 * Ref callback for the node whose `innerHTML` is message HTML (`html`). Every component that
 * inserts message HTML uses it, so spoilers reveal the same way everywhere. It binds when the
 * node mounts, so a node mounted again with the same markup (an edit cancelled) is covered and
 * revealable too. When `html` changes, the callback changes with it and binds the new markup.
 * Every spoiler span has the `spoiler` class (`[data-spoiler]` is the same span).
 */
export function useSpoilerReveal(html: string): RefCallback<HTMLElement> {
  // The callback reads `html`, so the React Compiler keeps it as a dependency too. Markup with no
  // spoiler needs no listeners.
  return useCallback(
    (node: HTMLElement | null) =>
      node === null || !html.includes("spoiler") ? undefined : bindSpoilers(node),
    [html],
  );
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

/**
 * The covered spoiler `target` is in, or null. A spoiler can sit inside another (stored HTML from
 * elsewhere): the outermost covered one is revealed first.
 */
function hiddenSpoiler(target: EventTarget | null): HTMLElement | null {
  let found: HTMLElement | null = null;

  for (let spoiler = spoilerElement(target); spoiler !== null; ) {
    if (!spoiler.hasAttribute("data-revealed")) {
      found = spoiler;
    }

    spoiler = spoilerElement(spoiler.parentElement);
  }

  return found;
}

/** Whether `node` is inside a spoiler under `spoiler` that is still covered. */
function stillCovered(node: Element, spoiler: HTMLElement): boolean {
  for (let parent = node.parentElement; parent !== null && parent !== spoiler; ) {
    if (parent.matches(SPOILER) && !parent.hasAttribute("data-revealed")) {
      return true;
    }

    parent = parent.parentElement;
  }

  return false;
}

/**
 * Shows this spoiler's text and gives its contents their names and focus back. Contents of a
 * spoiler inside it that is still covered stay hidden until that one is revealed.
 */
function reveal(spoiler: HTMLElement): void {
  spoiler.setAttribute("data-revealed", "");
  spoiler.removeAttribute("aria-label");
  spoiler.removeAttribute("role");
  spoiler.removeAttribute("tabindex");

  for (const node of spoiler.querySelectorAll<HTMLElement>("*")) {
    if (stillCovered(node, spoiler)) {
      continue;
    }

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
