/**
 * Where keyboard focus goes in the sidebar once a change has redrawn it: a row's or category's
 * drag handle (`data-drag-handle`), or a section's heading (`data-drop-section`).
 */

/** A row or category's drag handle where it shows (a folded section's own list is inert). */
export function liveHandle(container: HTMLElement | null, key: string): HTMLElement | null {
  return (
    [...(container?.querySelectorAll<HTMLElement>(`[data-drag-handle="${key}"]`) ?? [])].find(
      (candidate) => candidate.closest("[inert]") === null,
    ) ?? null
  );
}

/** The toggle button in a section's heading, by section key. */
export function headingTrigger(container: HTMLElement | null, key: string): HTMLElement | null {
  return (
    container?.querySelector<HTMLElement>(
      `[data-drop-section="${key}"] .sidebar-section-trigger`,
    ) ?? null
  );
}

/**
 * Where focus follows a row that moved to another section: its handle, or, when that section is
 * folded away, the section's heading (`section` names it in case the folded list isn't drawn).
 */
export function movedRowFocus(
  container: HTMLElement | null,
  key: string,
  section: string | null,
): HTMLElement | null {
  const live = liveHandle(container, key);

  if (live !== null) {
    return live;
  }

  const folded = container
    ?.querySelector(`[data-drag-handle="${key}"]`)
    ?.closest<HTMLElement>("[data-drop-section]")?.dataset.dropSection;

  const holder = folded ?? section;

  return holder === null || holder === undefined ? null : headingTrigger(container, holder);
}
