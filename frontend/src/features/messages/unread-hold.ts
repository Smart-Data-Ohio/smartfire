/**
 * Rooms the viewer just marked unread from a message. While a room is held, its timeline doesn't
 * mark it read again on its own (it would undo the action at once); leaving the room lets go.
 */
const held = new Set<number>();

export function holdUnread(roomId: number): void {
  held.add(roomId);
}

export function releaseUnread(roomId: number): void {
  held.delete(roomId);
}

export function isUnreadHeld(roomId: number): boolean {
  return held.has(roomId);
}
