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

interface OfferSource {
  readonly status: string;
  readonly noteToSelfRoomId: number | null;
  readonly rows: Readonly<Record<number, DirectSource & { readonly membership: Involved }>>;
}

interface Involved {
  readonly involvement: string;
}

/**
 * Whether the sidebar offers your note-to-self: once it has loaded, whenever no visible row is it.
 * Worked out from the rows rather than kept in the placeholder list, so no order of snapshots and
 * removals can drop both the row and the offer.
 */
export function sidebarOffersNoteToSelf(sidebar: OfferSource, viewerId: number | null): boolean {
  if (viewerId === null || sidebar.status !== "ready") {
    return false;
  }

  return !Object.entries(sidebar.rows).some(
    ([id, row]) =>
      row.membership.involvement !== "invisible" &&
      (Number(id) === sidebar.noteToSelfRoomId || isNoteToSelf(row)),
  );
}
