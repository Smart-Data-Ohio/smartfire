/**
 * The viewer's ignore set: the one place the client keeps who they ignore. The write's reply, the
 * `blocks.updated` sync event and the `GET /blocks` snapshot all land here. Anything that shows
 * ignore state reads it through `isIgnored`; a payload's own `ignored` / `ignoredByMe` flag
 * counts only until a snapshot has loaded, so a late reply can't undo a newer change.
 *
 * Revisions are the viewer's `activity_revision`. The server bumps it in the same transaction as
 * any block row it actually changes, and reads it back in that transaction (a write's reply) or in
 * the same read snapshot as the rows (`GET /blocks`). A write that changes nothing returns the
 * current value. So "person P at revision R" has exactly one answer, but one revision can carry
 * several people's states: a change is ordered against what's held for that person, and a
 * snapshot against the last snapshot.
 */
import type { BlockList } from "../gen/BlockList.ts";
import type { BlockState } from "../gen/BlockState.ts";

/** What the set knows about one person, and as of which revision. */
export interface IgnoreEntry {
  readonly ignored: boolean;
  readonly revision: number;
}

export interface IgnoreSet {
  /**
   * The revision of the last snapshot applied, or null before one. From then on, someone with no
   * entry isn't ignored, as of this revision.
   */
  readonly snapshot: number | null;
  /** Ignored people from the snapshot, and any person a newer change has landed for. */
  readonly people: Readonly<Record<number, IgnoreEntry>>;
  /**
   * Counts the moments a change may have been missed: a `blocks.updated` (the refetch it asks
   * for), a reconnect that can't replay, a refetch that failed.
   */
  readonly invalidatedAt: number;
  /**
   * The newest `invalidatedAt` a landed snapshot covers: the value when its request started, so
   * a reply to an older request can't vouch for a later miss. Only moves forward.
   */
  readonly coveredThrough: number;
}

export const emptyIgnores: IgnoreSet = {
  snapshot: null,
  people: {},
  invalidatedAt: 0,
  coveredThrough: 0,
};

/** A change may be missing: the next chance (a resume, an event) refetches the snapshot. */
export function ignoresStale(set: IgnoreSet): boolean {
  return set.coveredThrough < set.invalidatedAt;
}

/** The revision the set knows `userId` as of: their own entry's, else the snapshot's. */
function knownAt(set: IgnoreSet, userId: number): number {
  return set.people[userId]?.revision ?? set.snapshot ?? Number.NEGATIVE_INFINITY;
}

/**
 * One person's change, from a write's reply or the sync event. It lands when it's newer than what
 * the set knows about that person. One at the same revision states the same fact (the revision
 * only moves with a change), so the echo of a write changes nothing.
 */
export function applyBlockChange(set: IgnoreSet, change: BlockState): IgnoreSet {
  if (change.revision <= knownAt(set, change.userId)) {
    return set;
  }

  return {
    ...set,
    people: {
      ...set.people,
      [change.userId]: { ignored: change.ignoredByMe, revision: change.revision },
    },
  };
}

/**
 * A `GET /blocks` snapshot, from a request started when the set's `invalidatedAt` was
 * `coveredAt`. Every person known as of its revision or earlier takes its answer (listed: ignored;
 * absent: not); a change newer than it is kept. Data older than the last snapshot applied is
 * dropped, but the reply still covers what was invalidated before its request: what the set holds
 * is newer still.
 */
export function applyBlockSnapshot(
  set: IgnoreSet,
  snapshot: BlockList,
  coveredAt: number,
): IgnoreSet {
  const coveredThrough = Math.max(set.coveredThrough, coveredAt);

  if (set.snapshot !== null && snapshot.revision < set.snapshot) {
    return coveredThrough === set.coveredThrough ? set : { ...set, coveredThrough };
  }

  const people: Record<number, IgnoreEntry> = {};

  for (const [id, entry] of Object.entries(set.people)) {
    if (entry.revision > snapshot.revision) {
      people[Number(id)] = entry;
    }
  }

  for (const person of snapshot.people) {
    people[person.user.id] ??= { ignored: true, revision: snapshot.revision };
  }

  return { ...set, snapshot: snapshot.revision, people, coveredThrough };
}

/** A change may have been missed: the set is stale until a snapshot requested after this lands. */
export function invalidateIgnores(set: IgnoreSet): IgnoreSet {
  return { ...set, invalidatedAt: set.invalidatedAt + 1 };
}

/**
 * Whether the viewer ignores `userId`: the set's answer once it has one for them (or has loaded,
 * when missing means no), else `fallback`, the flag the payload being shown came with.
 */
export function isIgnored(
  set: IgnoreSet,
  userId: number | null,
  fallback: boolean | undefined,
): boolean {
  if (userId !== null) {
    const held = set.people[userId];

    if (held !== undefined) {
      return held.ignored;
    }

    if (set.snapshot !== null) {
      return false;
    }
  }

  return fallback ?? false;
}

/** Whether a message shows collapsed: its author, as the set has them. */
export function messageIgnored(
  set: IgnoreSet,
  message: { readonly creatorId: number; readonly ignored?: boolean | undefined },
): boolean {
  return isIgnored(set, message.creatorId, message.ignored);
}
