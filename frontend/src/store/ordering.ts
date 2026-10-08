/** Ordering and merging helpers the reducers share. */
import {
  nextObservation,
  observationOf,
  observeObject,
  rowObservationOf,
} from "../lib/request-observation.ts";
import type { MessageDTO, User } from "./model.ts";
import { landsOver, sameRecord } from "./revision.ts";
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

/** The users-row fields of two copies agree (everything but the observation-merged ones). */
export function sameUserRow(left: User, right: User): boolean {
  return (
    left.name === right.name &&
    left.role === right.role &&
    left.status === right.status &&
    left.bio === right.bio &&
    left.avatarUrl === right.avatarUrl &&
    left.createdAt === right.createdAt &&
    left.updatedAt === right.updatedAt
  );
}

/**
 * User row fields compare server revisions. The classic `updated_at` keeps Rails' `now` stamp,
 * so a repeated or regressed server clock can give two different rows one revision (a ban and
 * its unban); at an equal revision the row whose request or event was observed last wins.
 * Status expiry, avatar metadata and agent badges depend on other state, so their newest
 * request/event observation wins independently. An unchanged observation still advances
 * history, without mutating a held snapshot.
 */
export function mergeUserList(users: State["users"], list: readonly User[]): State["users"] {
  let next: Record<number, User> | null = null;

  for (const user of list) {
    const held = (next ?? users)[user.id];
    const supplied = observationOf(user);
    const observation = supplied ?? nextObservation();
    const previous = held === undefined ? 0 : (observationOf(held) ?? 0);
    const presentationCurrent = observation >= previous;
    const revision = landsOver(held, user) ? user : (held ?? user);
    const presentation = presentationCurrent ? user : (held ?? user);

    const sameRevision = held !== undefined && held.updatedAt === user.updatedAt;
    const previousRow = held === undefined ? 0 : (rowObservationOf(held) ?? 0);
    const incomingRow = rowObservationOf(user) ?? observation;
    const rowCurrent = !sameRevision || incomingRow >= previousRow;
    // The row's own fields, too: at a tie the later observation, otherwise the later revision.
    const rowPresentation = sameRevision ? (rowCurrent ? user : held) : revision;

    const row =
      held === undefined || user.updatedAt > held.updatedAt
        ? incomingRow
        : sameRevision
          ? Math.max(incomingRow, previousRow)
          : previousRow;

    const previousBadge = held?.agent == null ? previous : (observationOf(held.agent) ?? previous);

    const incomingBadge =
      user.agent === null ? observation : (observationOf(user.agent) ?? observation);

    const badgeCurrent = incomingBadge >= previousBadge;
    const badge = badgeCurrent ? user.agent : (held?.agent ?? null);
    const badgeAt = Math.max(previousBadge, incomingBadge);

    const agent =
      rowPresentation.role !== "bot" || badge === null
        ? null
        : observationOf(badge) === badgeAt
          ? badge
          : observeObject({ ...badge }, badgeAt);

    // Custom status and avatar icon follow the row presentation; the rest have their own order.
    const merged = { ...rowPresentation, hasAvatar: presentation.hasAvatar, agent };

    if (held !== undefined && sameRecord(held, merged)) {
      if (
        (supplied === undefined && user.updatedAt < held.updatedAt) ||
        (observation <= previous && row === previousRow && incomingBadge <= previousBadge)
      ) {
        continue;
      }
    }

    next ??= { ...users };
    next[user.id] = observeObject(merged, Math.max(observation, previous), row);
  }

  return next ?? users;
}
