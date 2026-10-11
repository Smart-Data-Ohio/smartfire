/**
 * The viewer's ignore set: the one place the client keeps who they ignore. The write's reply, the
 * `blocks.updated` sync event and the `GET /blocks` snapshot all land here, ordered by the
 * server's revision (the viewer's `activity_revision`, which every change moves). Anything that
 * shows ignore state reads it through `isIgnored`; a payload's own `ignored` / `ignoredByMe` flag
 * counts only until a snapshot has loaded, so a late reply can't undo a newer change.
 */
import type { BlockList } from "../gen/BlockList.ts";
import type { BlockState } from "../gen/BlockState.ts";

export interface IgnoreSet {
  /** A snapshot has landed: from then on, someone missing from `people` isn't ignored. */
  readonly loaded: boolean;
  /** The newest revision applied, from a write, an event or a snapshot; -1 before any. */
  readonly revision: number;
  /** Whether the viewer ignores each person, by user id. */
  readonly people: Readonly<Record<number, boolean>>;
}

export const emptyIgnores: IgnoreSet = { loaded: false, revision: -1, people: {} };

/**
 * One change, from a write's reply or the sync event. One no newer than the set is dropped: a
 * revision only moves with a change, so one the set already holds is the state it has, and the
 * event that echoes a write changes nothing.
 */
export function applyBlockChange(set: IgnoreSet, change: BlockState): IgnoreSet {
  if (change.revision <= set.revision) {
    return set;
  }

  return {
    ...set,
    revision: change.revision,
    people: { ...set.people, [change.userId]: change.ignoredByMe },
  };
}

/** A `GET /blocks` snapshot replaces the whole set at its revision, unless the set is newer. */
export function applyBlockSnapshot(set: IgnoreSet, snapshot: BlockList): IgnoreSet {
  if (snapshot.revision < set.revision) {
    return set;
  }

  const people: Record<number, boolean> = {};

  for (const person of snapshot.people) {
    people[person.user.id] = true;
  }

  return { loaded: true, revision: snapshot.revision, people };
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
      return held;
    }

    if (set.loaded) {
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
