/** Shared ordering for reads, local writes, live copies and list membership. */
export interface Freshness {
  readonly clock: number;
  readonly marks: Readonly<Record<string, { readonly at: number; readonly local: boolean }>>;
  readonly reads: Readonly<
    Record<number, { readonly list: string | null; readonly rejected: boolean }>
  >;
  readonly deltas: readonly MembershipDelta[];
}

interface MembershipDelta {
  readonly list: string;
  readonly id: number;
  readonly added: boolean;
  readonly at: number;
}

export const emptyFreshness: Freshness = { clock: 0, marks: {}, reads: {}, deltas: [] };

export function startRead(state: Freshness, list: string | null = null) {
  const ticket = state.clock + 1;

  return {
    freshness: {
      ...state,
      clock: ticket,
      reads: { ...state.reads, [ticket]: { list, rejected: false } },
    },
    ticket,
  };
}

/** Finishing failures and interrupted reads prunes their history too. */
export function finishRead(state: Freshness, ticket: number) {
  const { [ticket]: settled, ...reads } = state.reads;

  return {
    freshness: {
      ...state,
      reads,
      deltas: state.deltas.filter((delta) =>
        Object.entries(reads).some(
          ([ticket, read]) => read.list === delta.list && Number(ticket) < delta.at,
        ),
      ),
    },
    rejected: settled?.rejected ?? false,
  };
}

export function markLocal(state: Freshness, key: string): Freshness {
  const at = state.clock + 1;

  return { ...state, clock: at, marks: { ...state.marks, [key]: { at, local: true } } };
}

export function modifiedAt(state: Freshness, key: string): number {
  return state.marks[key]?.at ?? 0;
}

/** A derived snapshot caught up to this key's revision. */
export function acknowledge(state: Freshness, key: string, at: number): Freshness {
  return { ...state, marks: { ...state.marks, [key]: { at, local: false } } };
}

/** Wire records here contain scalar facts; property order does not affect equality. */
export function sameRecord<T extends object>(left: T, right: T): boolean {
  const entries = Object.entries(left);
  const other = new Map(Object.entries(right));

  return entries.length === other.size && entries.every(([key, value]) => other.get(key) === value);
}

interface Copies<T> {
  readonly held: T | undefined;
  readonly incoming: T;
  readonly same: (left: T, right: T) => boolean;
  readonly timestamp?: (value: T) => string | null;
  /** Domain rules such as a confirmed decision never returning to pending. */
  readonly keep?: (held: T, incoming: T) => boolean;
}

function keeps<T>(state: Freshness, key: string, ticket: number, copies: Copies<T>): boolean {
  const { held, incoming, timestamp, keep } = copies;

  if (held !== undefined) {
    if (keep?.(held, incoming)) {
      return true;
    }

    const heldAt = state.marks[key]?.local ? null : (timestamp?.(held) ?? null);
    const incomingAt = timestamp?.(incoming) ?? null;

    if (heldAt !== null && incomingAt !== null && heldAt !== incomingAt) {
      return heldAt > incomingAt;
    }
  }

  return modifiedAt(state, key) > ticket;
}

function rejectedRead(state: Freshness, ticket: number): Freshness {
  const read = state.reads[ticket];

  return read === undefined
    ? state
    : { ...state, reads: { ...state.reads, [ticket]: { ...read, rejected: true } } };
}

/** A response merges one key. The read accumulates partial rejection until finishRead. */
export function land<T>(state: Freshness, key: string, ticket: number, copies: Copies<T>) {
  if (keeps(state, key, ticket, copies)) {
    return {
      freshness: modifiedAt(state, key) > ticket ? rejectedRead(state, ticket) : state,
      value: copies.held,
      accepted: false,
    };
  }

  return {
    freshness: {
      ...state,
      marks: {
        ...state.marks,
        [key]: { at: Math.max(ticket, modifiedAt(state, key)), local: false },
      },
    },
    value: copies.incoming,
    accepted: true,
  };
}

/** Equal echoes confirm local data without invalidating the write's own read. */
export function receive<T>(state: Freshness, key: string, copies: Copies<T>) {
  const at = state.clock + 1;

  if (keeps(state, key, at, copies)) {
    return { freshness: state, value: copies.held, accepted: false };
  }

  if (copies.held !== undefined && copies.same(copies.held, copies.incoming)) {
    return {
      freshness: state.marks[key]?.local
        ? {
            ...state,
            marks: { ...state.marks, [key]: { at: modifiedAt(state, key), local: false } },
          }
        : state,
      value: copies.held,
      accepted: true,
    };
  }

  return {
    freshness: { ...state, clock: at, marks: { ...state.marks, [key]: { at, local: false } } },
    value: copies.incoming,
    accepted: true,
  };
}

/** Existing members keep their position, including the last row of a paged window. */
export function placeId(
  ids: readonly number[],
  id: number,
  belongs: boolean,
  order: (left: number, right: number) => number,
): readonly number[] {
  if (!belongs) {
    return ids.includes(id) ? ids.filter((held) => held !== id) : ids;
  }

  if (ids.includes(id)) {
    return ids;
  }

  const index = ids.findIndex((held) => order(id, held) < 0);

  return index < 0 ? [...ids, id] : ids.toSpliced(index, 0, id);
}

/** Record only real membership changes, and only while a read can need them. */
export function membership(
  state: Freshness,
  list: string,
  before: readonly number[],
  after: readonly number[],
): Freshness {
  const at = state.clock + 1;

  const deltas = [
    ...before.flatMap((id) => (after.includes(id) ? [] : [{ list, id, added: false, at }])),
    ...after.flatMap((id) => (before.includes(id) ? [] : [{ list, id, added: true, at }])),
  ];

  if (deltas.length === 0) {
    return state;
  }

  return {
    ...state,
    clock: at,
    deltas: Object.values(state.reads).some((read) => read.list === list)
      ? [...state.deltas, ...deltas]
      : state.deltas,
  };
}

export function replay(
  state: Freshness,
  list: string,
  ticket: number,
  pageIds: readonly number[],
  order: (left: number, right: number) => number,
) {
  const deltas = state.deltas.filter((delta) => delta.list === list && delta.at > ticket);
  const ids = deltas.reduce((held, delta) => placeId(held, delta.id, delta.added, order), pageIds);

  return { freshness: deltas.length === 0 ? state : rejectedRead(state, ticket), ids };
}
