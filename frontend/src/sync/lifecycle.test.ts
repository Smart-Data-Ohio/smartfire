import { afterEach, expect, it } from "@effect/vitest";
import { Effect, Ref, Stream } from "effect";
import { vi } from "vitest";
import { Lifecycle } from "./lifecycle.ts";

afterEach(() => vi.restoreAllMocks());

for (const type of ["pointerdown", "keydown", "wheel", "input"]) {
  it.effect(`ignores a synthetic ${type} dispatched on the document`, () =>
    Effect.gen(function* () {
      vi.spyOn(document, "visibilityState", "get").mockReturnValue("visible");

      const lifecycle = yield* Lifecycle;
      const inputs = yield* Ref.make(0);
      const listener = vi.spyOn(document, "addEventListener");

      yield* Effect.forkChild(
        lifecycle.input.pipe(Stream.runForEach(() => Ref.update(inputs, (count) => count + 1))),
      );
      yield* Effect.promise(() =>
        vi.waitFor(() => expect(listener.mock.calls.some(([name]) => name === type)).toBe(true)),
      );
      yield* Effect.sync(() => document.dispatchEvent(new Event(type, { bubbles: true })));
      yield* Effect.yieldNow.pipe(Effect.repeat({ times: 20 }));

      expect(yield* Ref.get(inputs)).toBe(0);
    }).pipe(Effect.provide(Lifecycle.layerBrowser)),
  );
}
