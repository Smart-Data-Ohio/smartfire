/**
 * Where `/r/:urlRoomId/t/:threadId` should go when the thread lives in another room.
 * `null` when the URL's room is already the thread's.
 */
export function foreignThreadHref(
  urlRoomId: number,
  thread: { readonly id: number; readonly roomId: number },
  focusMessageId: number | null,
): string | null {
  if (thread.roomId === urlRoomId) {
    return null;
  }

  const query = focusMessageId === null ? "" : `?m=${focusMessageId}`;

  return `/r/${thread.roomId}/t/${thread.id}${query}`;
}
