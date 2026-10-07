/**
 * Orders replies that land the same records: each request takes a number when it starts, and a
 * record lands only from a reply that started no earlier than the last one to land it. A slow
 * load that began before a write cannot undo the write's reply.
 */
export interface LandingOrder {
  /** The number for a request starting now. */
  readonly start: () => number;
  /** The keys a reply started at `started` may still land, recorded as landed by it. */
  readonly claim: <K>(started: number, keys: readonly K[], keyOf: (item: K) => number) => K[];
}

/** A fresh order, with nothing landed yet. */
export function landingOrder(): LandingOrder {
  let next = 0;
  const landed = new Map<number, number>();

  return {
    start: () => ++next,
    claim: (started, items, keyOf) => {
      const fresh = items.filter((item) => (landed.get(keyOf(item)) ?? 0) <= started);

      for (const item of fresh) {
        landed.set(keyOf(item), started);
      }

      return fresh;
    },
  };
}
