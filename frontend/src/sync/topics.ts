import { Context, Effect, Layer, Ref } from "effect";
import type { ClientFrame } from "../gen/ClientFrame.ts";
import { SyncLink } from "./link.ts";

/** How many released room topics stay subscribed, so going back to a room is instant. */
export const HOT_ROOMS = 8;

interface Interest {
  /** Holders per topic (a mounted room view, say). */
  readonly holders: ReadonlyMap<string, number>;
  /** Released room topics still subscribed, least recently released first. */
  readonly hot: readonly string[];
}

const isRoomTopic = (topic: string) => topic.startsWith("room:");

function acquireIn(interest: Interest, topic: string): readonly [ClientFrame | null, Interest] {
  const held = interest.holders.get(topic) ?? 0;
  const subscribed = held > 0 || interest.hot.includes(topic);

  const next: Interest = {
    holders: new Map(interest.holders).set(topic, held + 1),
    hot: interest.hot.filter((hot) => hot !== topic),
  };

  return [subscribed ? null : { t: "sub", topics: [topic] }, next];
}

function releaseIn(interest: Interest, topic: string): readonly [ClientFrame | null, Interest] {
  const held = interest.holders.get(topic) ?? 0;

  if (held === 0) {
    return [null, interest];
  }

  const holders = new Map(interest.holders);

  if (held > 1) {
    return [null, { ...interest, holders: holders.set(topic, held - 1) }];
  }

  holders.delete(topic);

  if (!isRoomTopic(topic)) {
    return [
      { t: "unsub", topics: [topic] },
      { ...interest, holders },
    ];
  }

  const hot = [...interest.hot, topic];
  const evicted = hot.length > HOT_ROOMS ? hot.slice(0, hot.length - HOT_ROOMS) : [];

  return [
    evicted.length === 0 ? null : { t: "unsub", topics: evicted },
    { holders, hot: hot.slice(evicted.length) },
  ];
}

/**
 * Ref-counted interest in sync topics (`room:<id>`, `thread:<id>`; `user` is implicit). The first
 * holder subscribes; the last release unsubscribes, except that the last `HOT_ROOMS` released
 * rooms stay subscribed. `sub`/`unsub` go out while connected; `hello` carries the whole set.
 */
export class Topics extends Context.Service<
  Topics,
  {
    readonly acquire: (topic: string) => Effect.Effect<void>;
    readonly release: (topic: string) => Effect.Effect<void>;
    /**
     * Forgets a room topic the server refused before the membership existed. A hot entry would
     * otherwise count as subscribed, so coming back would not send `sub` again. A view that
     * still holds the topic subscribes now.
     */
    readonly forgetRejected: (topic: string) => Effect.Effect<void>;
    /** Every subscribed topic: held ones, then hot ones. */
    readonly subscribed: Effect.Effect<readonly string[]>;
  }
>()("smartfire/sync/Topics") {
  static readonly layer = Layer.effect(
    Topics,
    Effect.gen(function* () {
      const link = yield* SyncLink;
      const interest = yield* Ref.make<Interest>({ holders: new Map(), hot: [] });

      const change =
        (step: (interest: Interest, topic: string) => readonly [ClientFrame | null, Interest]) =>
        (topic: string) =>
          Effect.flatMap(
            Ref.modify(interest, (current) => step(current, topic)),
            (frame) => (frame === null ? Effect.void : link.send(frame)),
          );

      const forgetRejected = (topic: string) =>
        Effect.flatMap(
          Ref.modify(interest, (current) => {
            const held = (current.holders.get(topic) ?? 0) > 0;
            const frame: ClientFrame | null = held ? { t: "sub", topics: [topic] } : null;

            const next: Interest = {
              holders: current.holders,
              hot: current.hot.filter((hot) => hot !== topic),
            };

            return [frame, next] as const;
          }),
          (frame) => (frame === null ? Effect.void : link.send(frame)),
        );

      return Topics.of({
        acquire: change(acquireIn),
        release: change(releaseIn),
        forgetRejected,
        subscribed: Effect.map(Ref.get(interest), (current) => [
          ...current.holders.keys(),
          ...current.hot,
        ]),
      });
    }),
  );
}
