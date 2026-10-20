import { Context, Effect, Layer, Stream } from "effect";

export class Lifecycle extends Context.Service<
  Lifecycle,
  {
    /** Moments worth reconnecting at once: back online, or the tab shown again. */
    readonly wakeups: Stream.Stream<void>;
    /** `true` when the tab is shown, `false` when hidden, on every change. */
    readonly visibility: Stream.Stream<boolean>;
    readonly isVisible: Effect.Effect<boolean>;
    readonly input: Stream.Stream<void>;
  }
>()("smartfire/sync/Lifecycle") {
  static readonly layerBrowser = Layer.sync(Lifecycle, () => {
    const isVisible = () => document.visibilityState === "visible";

    const visibility = Stream.fromEventListener<Event>(document, "visibilitychange").pipe(
      Stream.map(isVisible),
    );

    const online = Stream.fromEventListener<Event>(window, "online").pipe(Stream.as(true));

    const input = Stream.mergeAll(
      ["pointerdown", "keydown", "wheel", "input"].map((type) =>
        Stream.fromEventListener<Event>(document, type, { capture: true, passive: true }),
      ),
      { concurrency: "unbounded" },
    ).pipe(
      Stream.filter((event) => event.isTrusted && isVisible()),
      Stream.as(undefined),
    );

    return {
      wakeups: Stream.merge(online, Stream.filter(visibility, Boolean)).pipe(Stream.as(undefined)),
      visibility,
      isVisible: Effect.sync(isVisible),
      input,
    };
  });
}
