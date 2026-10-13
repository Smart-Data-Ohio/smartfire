import { NOTES_FALLBACK, ownName, YOU_TAG } from "./note-to-self.ts";

/**
 * Your name with a muted "(you)" after it, or `fallback` alone ("Your notes", or "You" for a
 * person) when you have no name to show: an empty or blank one never leaves a bare "(you)".
 */
export function YouLabel({
  name,
  tagClassName,
  fallback = NOTES_FALLBACK,
}: {
  readonly name: string | null | undefined;
  readonly tagClassName: string;
  readonly fallback?: string;
}) {
  const own = ownName(name);

  if (own === null) {
    return <>{fallback}</>;
  }

  return (
    <>
      {own}
      <span className={tagClassName}> {YOU_TAG}</span>
    </>
  );
}
