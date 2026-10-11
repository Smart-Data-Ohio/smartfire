import { afterEach, beforeEach, describe, expect, it } from "@effect/vitest";
import { Deferred, Effect, Fiber, Layer, Schema } from "effect";
import * as HttpClient from "effect/http/HttpClient";
import * as HttpClientRequest from "effect/http/HttpClientRequest";
import * as HttpClientResponse from "effect/http/HttpClientResponse";
import { ApiClient, ApiConfig, type ApiRequest, Navigation } from "../api/client.ts";
import { ApiErrorResponse, SudoRequired } from "../api/errors.ts";
import type { SudoMethod } from "../gen/SudoMethod.ts";
import {
  type ConfirmationGate,
  KEPT_KEY,
  makeConfirmationGate,
  resumeAfterGoogle,
  spaReturnPath,
  startGoogle,
  submit,
} from "./reauthentication.ts";

/** What the fake server saw. */
interface Seen {
  readonly method: string;
  readonly path: string;
  readonly body: string;
}

interface Reply {
  readonly status: number;
  readonly body: unknown;
}

const encodeErrorBody = Schema.encodeSync(ApiErrorResponse);

const required = (methods: readonly SudoMethod[] = ["password", "totp"]) =>
  new SudoRequired({
    message: "Confirm your password to continue",
    reauthentication: { methods: [...methods], retry: null },
  });

/** The 403 a guarded write answers while the confirmation has lapsed. */
const held = (methods?: readonly SudoMethod[]): Reply => ({
  status: 403,
  body: encodeErrorBody({ error: required(methods) }),
});

const refused: Reply = {
  status: 401,
  body: { kind: "error", message: "Confirmation failed. Try again." },
};

const confirmed: Reply = { status: 200, body: { kind: "confirmed", retry: null } };

/**
 * The real `ApiClient` over a fake `HttpClient` (replies from `answer`, by request in order) and a
 * fresh confirmation gate on session storage.
 */
function harness(
  answer: (seen: Seen, index: number) => Reply,
  hold: (seen: Seen) => Effect.Effect<void> = () => Effect.void,
) {
  const seen: Seen[] = [];
  const gate = makeConfirmationGate(() => sessionStorage);

  const http = HttpClient.make((request, url) =>
    Effect.gen(function* () {
      const web = yield* HttpClientRequest.toWeb(request).pipe(Effect.orDie);
      const body = yield* Effect.promise(() => web.text());
      const entry: Seen = { method: request.method, path: url.pathname, body };

      seen.push(entry);

      const index = seen.length - 1;

      yield* hold(entry);

      const reply = answer(entry, index);

      return HttpClientResponse.fromWeb(
        request,
        new Response(JSON.stringify(reply.body), {
          status: reply.status,
          headers: { "content-type": "application/json" },
        }),
      );
    }),
  );

  const layer = ApiClient.layer.pipe(
    Layer.provide(
      Layer.mergeAll(
        Layer.succeed(HttpClient.HttpClient, http),
        ApiConfig.layer("/api/v1"),
        Layer.succeed(Navigation, {
          location: Effect.succeed("/app/admin/styles"),
          assign: () => Effect.void,
        }),
        gate.layer,
      ),
    ),
  );

  return { layer, seen, gate };
}

const anyJson = (json: Schema.Json) => Effect.succeed(json);

/** Sends `request` on the client under test. */
const send = (request: ApiRequest) =>
  Effect.gen(function* () {
    const client = yield* ApiClient;

    return yield* client.execute(request, anyJson);
  });

const styles: ApiRequest = {
  method: "PATCH",
  path: "/admin/custom_styles",
  body: { css: "body { color: red; }" },
};

const token: ApiRequest = {
  method: "PUT",
  path: "/settings/github_connection",
  body: { accessToken: "ghp_secret" },
  secret: true,
};

/** Lets forked writes run until the gate shows a prompt for `writes` of them. */
const untilPrompt = (gate: ConfirmationGate, writes = 1) =>
  Effect.gen(function* () {
    for (let turn = 0; turn < 100 && (gate.snapshot()?.writes ?? 0) < writes; turn += 1) {
      yield* Effect.sleep(1);
    }

    expect(gate.snapshot()?.writes).toBe(writes);
  });

const writesTo = (seen: readonly Seen[], path: string) =>
  seen.filter((entry) => entry.path === `/api/v1${path}`);

beforeEach(() => {
  const meta = document.createElement("meta");

  meta.name = "csrf-token";
  meta.content = "token";
  document.head.append(meta);
});

afterEach(() => {
  sessionStorage.clear();
  document.querySelector('meta[name="csrf-token"]')?.remove();
});

describe("the confirmation gate", () => {
  it.live("replays a held write exactly once after the person confirms", () => {
    const { layer, seen, gate } = harness((entry) =>
      entry.path.endsWith("/custom_styles") && seen.length === 1
        ? held()
        : { status: 200, body: { css: "body { color: red; }" } },
    );

    return Effect.gen(function* () {
      const write = yield* Effect.forkChild(send(styles));

      yield* untilPrompt(gate);
      expect(gate.snapshot()).toEqual({ methods: ["password", "totp"], writes: 1, secrets: 0 });

      gate.confirmed();

      expect(yield* Fiber.join(write)).toEqual({ css: "body { color: red; }" });
      expect(gate.snapshot()).toBeNull();
      expect(writesTo(seen, "/admin/custom_styles").map((entry) => entry.body)).toEqual([
        '{"css":"body { color: red; }"}',
        '{"css":"body { color: red; }"}',
      ]);
    }).pipe(Effect.provide(layer));
  });

  it.live("cancelling fails the write unsent with ConfirmationCancelled", () => {
    const { layer, seen, gate } = harness(() => held());

    return Effect.gen(function* () {
      const write = yield* Effect.forkChild(send(styles));

      yield* untilPrompt(gate);
      gate.cancel();

      const error = yield* Fiber.join(write).pipe(Effect.flip);

      expect(error._tag).toBe("ConfirmationCancelled");
      expect(gate.snapshot()).toBeNull();
      expect(seen).toHaveLength(1);
    }).pipe(Effect.provide(layer));
  });

  it.live("a wrong password keeps the write waiting; the right one replays it", () => {
    const { layer, seen, gate } = harness((entry) => {
      if (entry.path === "/api/v1/sudo") {
        return entry.body.includes("wrong") ? refused : confirmed;
      }

      return writesTo(seen, "/admin/custom_styles").length === 1
        ? held()
        : { status: 200, body: { css: "ok" } };
    });

    return Effect.gen(function* () {
      const write = yield* Effect.forkChild(send(styles));

      yield* untilPrompt(gate);

      expect(yield* submit(gate, { kind: "password", password: "wrong" })).toBe(
        "Confirmation failed. Try again.",
      );
      expect(gate.snapshot()?.writes).toBe(1);
      expect(yield* submit(gate, { kind: "password", password: "right" })).toBeNull();
      expect(yield* Fiber.join(write)).toEqual({ css: "ok" });
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(2);
    }).pipe(Effect.provide(layer));
  });

  it.live("a wrong authenticator code keeps the write waiting; a rate limit says so", () => {
    const { layer, seen, gate } = harness((entry) => {
      if (entry.path === "/api/v1/sudo") {
        if (entry.body.includes("limit")) {
          return {
            status: 429,
            body: { kind: "error", message: "Too many attempts. Try again in a few minutes." },
          };
        }

        return entry.body.includes("000000") ? refused : confirmed;
      }

      return writesTo(seen, "/admin/custom_styles").length === 1
        ? held(["totp"])
        : { status: 200, body: { css: "ok" } };
    });

    return Effect.gen(function* () {
      const write = yield* Effect.forkChild(send(styles));

      yield* untilPrompt(gate);
      expect(gate.snapshot()?.methods).toEqual(["totp"]);
      expect(yield* submit(gate, { kind: "totp", code: "000000" })).toBe(
        "Confirmation failed. Try again.",
      );
      expect(yield* submit(gate, { kind: "totp", code: "limit" })).toBe(
        "Too many attempts. Try again in a few minutes.",
      );
      expect(gate.snapshot()?.writes).toBe(1);
      expect(yield* submit(gate, { kind: "totp", code: "123456" })).toBeNull();
      expect(yield* Fiber.join(write)).toEqual({ css: "ok" });
    }).pipe(Effect.provide(layer));
  });

  it.live("a replay held again (the confirmation already expired) fails, not a second ask", () => {
    const { layer, seen, gate } = harness(() => held());

    return Effect.gen(function* () {
      const write = yield* Effect.forkChild(send(styles));

      yield* untilPrompt(gate);
      gate.confirmed();

      const error = yield* Fiber.join(write).pipe(Effect.flip);

      expect(error._tag).toBe("SudoRequired");
      expect(gate.snapshot()).toBeNull();
      expect(seen).toHaveLength(2);
    }).pipe(Effect.provide(layer));
  });

  it.live("concurrent held writes share one prompt and each replays once", () => {
    const { layer, seen, gate } = harness((entry) =>
      writesTo(seen, entry.path.slice("/api/v1".length)).length === 1
        ? held()
        : { status: 200, body: { path: entry.path } },
    );

    const role: ApiRequest = {
      method: "PATCH",
      path: "/admin/people/4",
      body: { role: "administrator" },
    };

    return Effect.gen(function* () {
      const first = yield* Effect.forkChild(send(styles));
      const second = yield* Effect.forkChild(send(role));

      yield* untilPrompt(gate, 2);
      gate.confirmed();

      expect(yield* Fiber.join(first)).toEqual({ path: "/api/v1/admin/custom_styles" });
      expect(yield* Fiber.join(second)).toEqual({ path: "/api/v1/admin/people/4" });
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(2);
      expect(writesTo(seen, "/admin/people/4")).toHaveLength(2);
    }).pipe(Effect.provide(layer));
  });

  it.live("a write held before a confirmation landed replays without asking again", () => {
    const role: ApiRequest = { method: "PATCH", path: "/admin/people/4", body: { role: "member" } };
    const latch = Deferred.makeUnsafe<void>();
    const asked: number[] = [];

    const { layer, seen, gate } = harness(
      (entry) =>
        writesTo(seen, entry.path.slice("/api/v1".length)).length === 1
          ? held()
          : { status: 200, body: { path: entry.path } },
      // The role change's first answer is slow: it crosses the confirmation.
      (entry) =>
        entry.path.endsWith("/people/4") && writesTo(seen, "/admin/people/4").length === 1
          ? Deferred.await(latch)
          : Effect.void,
    );

    gate.subscribe(() => asked.push(gate.snapshot()?.writes ?? 0));

    return Effect.gen(function* () {
      const first = yield* Effect.forkChild(send(styles));

      yield* untilPrompt(gate);

      const second = yield* Effect.forkChild(send(role));

      yield* Effect.sleep(5);
      gate.confirmed();
      expect(yield* Fiber.join(first)).toEqual({ path: "/api/v1/admin/custom_styles" });

      Deferred.doneUnsafe(latch, Effect.void);
      expect(yield* Fiber.join(second)).toEqual({ path: "/api/v1/admin/people/4" });
      // One prompt, opened once and closed once: the late refusal never reopened it.
      expect(asked).toEqual([1, 0]);
      expect(writesTo(seen, "/admin/people/4")).toHaveLength(2);
    }).pipe(Effect.provide(layer));
  });

  it.live("a caller that gives up leaves the prompt; the others still wait", () => {
    const { layer, gate } = harness(() => held());

    return Effect.gen(function* () {
      const first = yield* Effect.forkChild(send(styles));

      const second = yield* Effect.forkChild(
        send({ ...styles, path: "/admin/workspace/join_code" }),
      );

      yield* untilPrompt(gate, 2);
      yield* Fiber.interrupt(first);
      expect(gate.snapshot()?.writes).toBe(1);
      yield* Fiber.interrupt(second);
      expect(gate.snapshot()).toBeNull();
    }).pipe(Effect.provide(layer));
  });
});

describe("the Google round trip", () => {
  it.live("keeps credential-free writes, fails secret ones, and replays the kept once", () => {
    let fresh = false;

    const { layer, seen, gate } = harness((entry) => {
      if (entry.path === "/api/v1/sudo/google") {
        fresh = true;

        return { status: 200, body: { kind: "navigate", location: "https://accounts.test/o" } };
      }

      if (entry.path === "/api/v1/sudo/continue") {
        return fresh
          ? {
              status: 200,
              body: {
                kind: "confirmed",
                retry: {
                  method: "PATCH",
                  path: "/api/v1/admin/custom_styles",
                  returnTo: "/app/admin/styles",
                },
              },
            }
          : { status: 403, body: { kind: "ready", reauthentication: required().reauthentication } };
      }

      return fresh ? { status: 200, body: { saved: true } } : held(["password", "google"]);
    });

    return Effect.gen(function* () {
      const kept = yield* Effect.forkChild(send(styles));
      const secret = yield* Effect.forkChild(send(token));

      yield* untilPrompt(gate, 2);
      expect(gate.snapshot()).toMatchObject({ methods: ["password", "google"], secrets: 1 });

      expect(yield* startGoogle(gate, "/app/admin/styles?tab=css")).toEqual({
        kind: "navigate",
        location: "https://accounts.test/o",
      });
      expect(gate.snapshot()).toBeNull();
      expect((yield* Fiber.join(secret).pipe(Effect.flip))._tag).toBe("SudoRequired");
      // The kept write waits for the page to go; the secret never reaches storage.
      expect(kept.pollUnsafe()).toBeUndefined();

      const stored = sessionStorage.getItem(KEPT_KEY) ?? "";

      expect(stored).not.toContain("ghp_secret");
      expect(JSON.parse(stored)).toEqual({
        returnTo: "/app/admin/styles?tab=css",
        writes: [styles],
        dropped: 1,
      });

      yield* Fiber.interrupt(kept);

      expect(yield* resumeAfterGoogle(gate)).toEqual({
        confirmed: true,
        returnTo: "/app/admin/styles",
        replayed: 1,
        failure: null,
        dropped: 1,
      });
      expect(sessionStorage.getItem(KEPT_KEY)).toBeNull();

      // A reload of the continue page sends nothing again.
      expect((yield* resumeAfterGoogle(gate)).replayed).toBe(0);
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(2);
    }).pipe(Effect.provide(layer));
  });

  it.live("a stale or refused Google return replays nothing and forgets the writes", () => {
    const { layer, seen, gate } = harness(() => ({
      status: 403,
      body: { kind: "ready", reauthentication: required().reauthentication },
    }));

    return Effect.gen(function* () {
      sessionStorage.setItem(
        KEPT_KEY,
        JSON.stringify({ returnTo: "/app/admin/people", writes: [styles], dropped: 0 }),
      );

      expect(yield* resumeAfterGoogle(gate)).toEqual({
        confirmed: false,
        returnTo: "/app/admin/people",
        replayed: 0,
        failure: null,
        dropped: 0,
      });
      expect(sessionStorage.getItem(KEPT_KEY)).toBeNull();
      expect(seen.map((entry) => entry.path)).toEqual(["/api/v1/sudo/continue"]);
    }).pipe(Effect.provide(layer));
  });

  it.live("a Google start the server refuses keeps the writes waiting", () => {
    const { layer, gate } = harness((entry) =>
      entry.path === "/api/v1/sudo/google"
        ? {
            status: 422,
            body: {
              kind: "error",
              message: "Google confirmation is not available for your account.",
            },
          }
        : held(),
    );

    return Effect.gen(function* () {
      const write = yield* Effect.forkChild(send(styles));

      yield* untilPrompt(gate);
      expect(yield* startGoogle(gate, "/app/admin/styles")).toEqual({
        kind: "error",
        message: "Google confirmation is not available for your account.",
      });
      expect(gate.snapshot()?.writes).toBe(1);
      expect(sessionStorage.getItem(KEPT_KEY)).toBeNull();
      yield* Fiber.interrupt(write);
    }).pipe(Effect.provide(layer));
  });

  it("only returns to SPA pages", () => {
    expect(spaReturnPath("/app/admin/styles?x=1")).toBe("/app/admin/styles?x=1");
    expect(spaReturnPath("/app")).toBe("/app");
    expect(spaReturnPath("//evil.test/app/")).toBe("/app/");
    expect(spaReturnPath("https://evil.test/app/")).toBe("/app/");
    expect(spaReturnPath("/account/edit")).toBe("/app/");
    expect(spaReturnPath("/app/sudo/continue")).toBe("/app/");
    expect(spaReturnPath(null)).toBe("/app/");
  });
});
