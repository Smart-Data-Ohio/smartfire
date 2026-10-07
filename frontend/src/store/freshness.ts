/** Client history only for list membership, which has no server revision. */
export interface Freshness {
  readonly clock: number;
  /** Outstanding reads by ticket; `reload` marks a first-page (re)load, not a next page. */
  readonly reads: Readonly<Record<number, { readonly list: string; readonly reload: boolean }>>;
  readonly deltas: readonly MembershipDelta[];
}

interface MembershipDelta {
  readonly list: string;
  readonly id: number;
  readonly added: boolean;
  readonly at: number;
}

export const emptyFreshness: Freshness = { clock: 0, reads: {}, deltas: [] };

function pruneHistory(state: Freshness, reads: Freshness["reads"]): Freshness {
  return {
    ...state,
    reads,
    deltas: state.deltas.filter((delta) =>
      Object.entries(reads).some(
        ([ticket, read]) => read.list === delta.list && Number(ticket) < delta.at,
      ),
    ),
  };
}

/**
 * A reload supersedes any earlier read of the same list. A next page supersedes nothing: it
 * never starts while a reload of its list is out (see `reloading`).
 */
export function startRead(state: Freshness, list: string, reload = true) {
  const ticket = state.clock + 1;
  const current = reload ? retireReads(state, list).freshness : state;

  return {
    freshness: {
      ...current,
      clock: ticket,
      reads: { ...current.reads, [ticket]: { list, reload } },
    },
    ticket,
  };
}

/** Whether a first-page (re)load of `list` is still on its way. */
export function reloading(state: Freshness, list: string): boolean {
  return Object.values(state.reads).some((read) => read.list === list && read.reload);
}

/** A view leaving retires its reads, even if their requests are still running. */
export function retireReads(state: Freshness, list: string) {
  const reads = Object.fromEntries(
    Object.entries(state.reads).filter(([, read]) => read.list !== list),
  );

  return { freshness: pruneHistory(state, reads) };
}

/** Finishing failures and interrupted reads prunes their history too. */
export function finishRead(state: Freshness, ticket: number) {
  const { [ticket]: _settled, ...reads } = state.reads;

  return { freshness: pruneHistory(state, reads) };
}

function compactDeltas(
  held: readonly MembershipDelta[],
  incoming: readonly MembershipDelta[],
): readonly MembershipDelta[] {
  const latest = new Map(held.map((delta) => [`${delta.list}:${delta.id}`, delta]));

  for (const delta of incoming) {
    latest.set(`${delta.list}:${delta.id}`, delta);
  }

  return [...latest.values()].sort((left, right) => left.at - right.at);
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
      ? compactDeltas(state.deltas, deltas)
      : state.deltas,
  };
}

/** Equal observations still establish membership while a page is in flight. */
export function observeMembership(
  state: Freshness,
  list: string,
  id: number,
  added: boolean,
): Freshness {
  if (!Object.values(state.reads).some((read) => read.list === list)) {
    return state;
  }

  const at = state.clock + 1;

  return {
    ...state,
    clock: at,
    deltas: compactDeltas(state.deltas, [{ list, id, added, at }]),
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

  return ids;
}
