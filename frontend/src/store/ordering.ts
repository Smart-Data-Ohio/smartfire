/** Ordering and merging helpers the reducers share. */
import type { MessageDTO, User } from "./model.ts";
import { landsOver } from "./revision.ts";
import type { State } from "./state.ts";

/** `(createdAt, id)`: the server's timeline order. */
export function compareMessages(left: MessageDTO, right: MessageDTO): number {
  if (left.createdAt !== right.createdAt) {
    return left.createdAt < right.createdAt ? -1 : 1;
  }

  return left.id - right.id;
}

/** Inserts `message` into ordered `ids` (no-op when present). */
export function insertOrdered(
  ids: readonly number[],
  message: MessageDTO,
  messages: Readonly<Record<number, MessageDTO>>,
): readonly number[] {
  if (ids.includes(message.id)) {
    return ids;
  }

  let low = 0;
  let high = ids.length;

  while (low < high) {
    const middle = (low + high) >>> 1;
    const other = messages[ids[middle] ?? -1];

    if (other !== undefined && compareMessages(other, message) < 0) {
      low = middle + 1;
    } else {
      high = middle;
    }
  }

  return [...ids.slice(0, low), message.id, ...ids.slice(low)];
}

/**
 * Lands `list` in `users`, each record only if it is at least as new as the one held
 * (`users.updated_at`), so a slow reply never undoes a newer one, whatever order they arrive in.
 * Answers `users` itself when nothing landed.
 */
export function mergeUserList(users: State["users"], list: readonly User[]): State["users"] {
  const landing = list.filter((user) => landsOver(users[user.id], user));

  if (landing.length === 0) {
    return users;
  }

  const next = { ...users };

  for (const user of landing) {
    next[user.id] = user;
  }

  return next;
}
