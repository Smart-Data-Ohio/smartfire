/** A sidebar row or room detail: the server marks the viewer's note-to-self on both. */
export interface DirectSource {
  readonly noteToSelf?: boolean;
}

/**
 * Whether this is the viewer's note-to-self (Slack's "message yourself"), as the server marks it:
 * the room opened for just them (its canonical member key). A group everyone else left is not
 * one, though only the viewer is left in it, so the SPA never works it out from the members.
 */
export function isNoteToSelf(source: DirectSource): boolean {
  return source.noteToSelf === true;
}
