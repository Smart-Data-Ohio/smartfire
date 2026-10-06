/**
 * The Popover API (Baseline 2024) puts menus, popovers, tooltips and toasts in the top layer, so
 * no z-index or portal is needed and `popover="auto"` brings light dismiss and Esc for free.
 * Where it is missing (old browsers, jsdom in unit tests) the element simply stays in the flow,
 * positioned by the anchor fallback, and the components handle Esc and outside clicks themselves.
 */
export function supportsPopover(element: HTMLElement): boolean {
  return "showPopover" in element && "hidePopover" in element;
}

function isOpen(element: HTMLElement): boolean {
  return element.matches(":popover-open");
}

export function showPopover(element: HTMLElement): void {
  if (supportsPopover(element) && element.isConnected && !isOpen(element)) {
    element.showPopover();
  }
}

export function hidePopover(element: HTMLElement): void {
  if (supportsPopover(element) && isOpen(element)) {
    element.hidePopover();
  }
}
