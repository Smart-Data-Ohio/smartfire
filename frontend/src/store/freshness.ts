/** Client history only for list membership, which has no server revision. */
export interface Freshness {
  readonly clock: number;
  readonly reads: Readonly<Record<number, { readonly list: string }>>;
  readonly deltas: readonly MembershipDelta[];
}

interface MembershipDelta {
  readonly list: string;
  readonly id: number;
  readonly added: boolean;
  readonly at: number;
}

export const emptyFreshness: Freshness = { clock: 0, reads: {}, deltas: [] };

export function startRead(state: Freshness, list: string) {
  const ticket = state.clock + 1;

  return {
    freshness: {
      ...state,
      clock: ticket,
      reads: { ...state.reads, [ticket]: { list } },
    },
    ticket,
  };
}

/** Finishing failures and interrupted reads prunes their history too. */
export function finishRead(state: Freshness, ticket: number) {
  const { [ticket]: _settled, ...reads } = state.reads;

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

  return ids;
}
