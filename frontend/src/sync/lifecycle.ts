import { Context, Effect, Layer, Stream } from "effect";

/**
 * The browser events the engine reacts to, behind a service so tests can fire them: coming back
 * online, and the tab being shown or hidden.
 */
export class Lifecycle extends Context.Service<
  Lifecycle,
  {
    /** Moments worth reconnecting at once: back online, or the tab shown again. */
    readonly wakeups: Stream.Stream<void>;
    /** `true` when the tab is shown, `false` when hidden, on every change. */
    readonly visibility: Stream.Stream<boolean>;
    readonly isVisible: Effect.Effect<boolean>;
  }
>()("smartfire/sync/Lifecycle") {
  static readonly layerBrowser = Layer.sync(Lifecycle, () => {
    const isVisible = () => document.visibilityState === "visible";

    const visibility = Stream.fromEventListener<Event>(document, "visibilitychange").pipe(
      Stream.map(isVisible),
    );

    const online = Stream.fromEventListener<Event>(window, "online").pipe(Stream.as(true));

    return {
      wakeups: Stream.merge(online, Stream.filter(visibility, Boolean)).pipe(Stream.as(undefined)),
      visibility,
      isVisible: Effect.sync(isVisible),
    };
  });
}
