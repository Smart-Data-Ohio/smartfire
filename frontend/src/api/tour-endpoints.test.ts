import { Effect, Exit } from "effect";
import { afterEach, describe, expect, it, vi } from "vitest";
import { FakeApi } from "./testing.ts";
import { completeTour, TOUR_PATH } from "./tour-endpoints.ts";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("completeTour", () => {
  it("patches classic's tour stamp with the session's CSRF token", async () => {
    const fetchStub = vi.fn(async () => new Response(null, { status: 204 }));

    vi.stubGlobal("fetch", fetchStub);

    await Effect.runPromise(completeTour().pipe(Effect.provide(FakeApi.layerClient)));

    expect(fetchStub).toHaveBeenCalledExactlyOnceWith(TOUR_PATH, {
      method: "PATCH",
      credentials: "same-origin",
      headers: { Accept: "application/json", "X-CSRF-Token": "test-csrf-token" },
    });
    expect(TOUR_PATH).toBe("/users/me/tour");
  });

  it("fails with the status when the server refuses", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(null, { status: 422 })),
    );

    const exit = await Effect.runPromiseExit(
      completeTour().pipe(Effect.provide(FakeApi.layerClient)),
    );

    expect(Exit.isFailure(exit)).toBe(true);
    expect(JSON.stringify(exit)).toContain("422");
  });
});
