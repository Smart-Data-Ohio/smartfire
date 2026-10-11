import { afterEach, beforeEach, describe, expect, it } from "@effect/vitest";
import { Deferred, Effect, Fiber, Layer, Option, Schema } from "effect";
import * as HttpClient from "effect/http/HttpClient";
import * as HttpClientRequest from "effect/http/HttpClientRequest";
import * as HttpClientResponse from "effect/http/HttpClientResponse";
import { ApiClient, ApiConfig, type ApiRequest, Navigation } from "../api/client.ts";
import { ApiErrorResponse, SudoRequired, TwoFactorRequired } from "../api/errors.ts";
import { onTwoFactorRequirement } from "../api/two-factor-gate.ts";
import type { SudoMethod } from "../gen/SudoMethod.ts";
import type { SudoRetry } from "../gen/SudoRetry.ts";
import {
  afterConfirming,
  type ConfirmationGate,
  confirmAgain,
  confirmFirst,
  confirmHere,
  discardKept,
  googleHere,
  KEPT_KEY,
  makeConfirmationGate,
  resumeAfterGoogle,
  spaReturnPath,
  startGoogle,
  submit,
  sudoStatus,
} from "./reauthentication.ts";

/** What the fake server saw. */
interface Seen {
  readonly method: string;
  readonly path: string;
  readonly query: URLSearchParams;
  readonly body: string;
}

interface Reply {
  readonly status: number;
  readonly body: unknown;
  /** The connection drops partway through the body: the status and headers arrive, then it ends. */
  readonly truncated?: boolean;
}

const encodeErrorBody = Schema.encodeSync(ApiErrorResponse);

/** The server's pending request for `seen`, as `require_sudo_mode` records it. */
const retryOf = (seen: Seen | null): SudoRetry | null =>
  seen === null ? null : { method: seen.method, path: seen.path, returnTo: "/app/admin/styles" };

const required = (
  methods: readonly SudoMethod[] = ["password", "totp"],
  seen: Seen | null = null,
) =>
  new SudoRequired({
    message: "Confirm your password to continue",
    reauthentication: { methods: [...methods], retry: retryOf(seen) },
  });

/** The 403 a guarded write (`seen`) answers while the confirmation has lapsed. */
const held = (methods?: readonly SudoMethod[], seen: Seen | null = null): Reply => ({
  status: 403,
  body: encodeErrorBody({ error: required(methods, seen) }),
});

const refused: Reply = {
  status: 401,
  body: { kind: "error", message: "Confirmation failed. Try again." },
};

const confirmed: Reply = { status: 200, body: { kind: "confirmed", retry: null } };

/** What the gate reads from the page: who is signed in, the clock, and the screen. */
interface Page {
  viewer: number | null;
  now: number;
  screen: string;
  /** Whether local storage takes writes (it reads either way). */
  markable: boolean;
  /** Whether session storage takes writes (it reads either way). */
  keepable: boolean;
}

/** `storage` as a browser that refuses to write it shows it: readable, every write throws. */
function unwritable(storage: Storage): Storage {
  const refuse = () => {
    throw new DOMException("The quota has been exceeded.", "QuotaExceededError");
  };

  return {
    get length() {
      return storage.length;
    },
    clear: refuse,
    getItem: (key) => storage.getItem(key),
    key: (index) => storage.key(index),
    removeItem: refuse,
    setItem: refuse,
  };
}

/**
 * One browser's Web Locks: a single holder per name, the others waiting their turn. A wait whose
 * `signal` aborts gives up (`null`) and never holds the lock.
 */
function browserLocks() {
  const tails = new Map<string, Promise<void>>();

  return (name: string, signal?: AbortSignal): Promise<(() => void) | null> => {
    const previous = tails.get(name) ?? Promise.resolve();
    const held = Promise.withResolvers<void>();

    tails.set(
      name,
      previous.then(() => held.promise),
    );

    return new Promise((acquired) => {
      signal?.addEventListener("abort", () => acquired(null), { once: true });
      void previous.then(() => {
        if (signal?.aborted === true) {
          held.resolve();
        } else {
          acquired(held.resolve);
        }
      });
    });
  };
}

/** A copy of `storage`, as duplicating a tab copies its session storage. */
function copyOf(storage: Storage): Storage {
  const items = new Map<string, string>();

  for (let index = 0; index < storage.length; index += 1) {
    const key = storage.key(index);

    if (key !== null) items.set(key, storage.getItem(key) ?? "");
  }

  return {
    get length() {
      return items.size;
    },
    clear: () => items.clear(),
    getItem: (key) => items.get(key) ?? null,
    key: (index) => [...items.keys()][index] ?? null,
    removeItem: (key) => {
      items.delete(key);
    },
    setItem: (key, value) => {
      items.set(key, value);
    },
  };
}

/**
 * The real `ApiClient` over a fake `HttpClient` (replies from `answer`, by request in order) and a
 * fresh confirmation gate on session storage, reading `page`. `duplicate` opens another tab of
 * the same browser on a copy of this one's session storage.
 */
function harness(
  answer: (seen: Seen, index: number) => Reply,
  hold: (seen: Seen) => Effect.Effect<void> = () => Effect.void,
  // The page's own Web Locks (`navigator.locks`) unless the test gives the browser's.
  lock:
    | ((name: string, signal?: AbortSignal) => Promise<(() => void) | null>)
    | "page" = browserLocks(),
) {
  const seen: Seen[] = [];

  const page: Page = {
    viewer: 7,
    now: 1_000_000,
    screen: "/app/admin/styles",
    markable: true,
    keepable: true,
  };

  const gateOn = (storage: Storage) => {
    const options = {
      storage: () => (page.keepable ? storage : unwritable(storage)),
      shared: () => (page.markable ? localStorage : unwritable(localStorage)),
      viewer: () => page.viewer,
      now: () => page.now,
      screen: () => page.screen,
    };

    return makeConfirmationGate(lock === "page" ? options : { ...options, lock });
  };

  const gate = gateOn(sessionStorage);

  const http = HttpClient.make((request, url) =>
    Effect.gen(function* () {
      const web = yield* HttpClientRequest.toWeb(request).pipe(Effect.orDie);
      const body = yield* Effect.promise(() => web.text());

      const entry: Seen = {
        method: request.method,
        path: url.pathname,
        query: url.searchParams,
        body,
      };

      seen.push(entry);

      const index = seen.length - 1;

      yield* hold(entry);

      const reply = answer(entry, index);

      const text = JSON.stringify(reply.body);

      return HttpClientResponse.fromWeb(
        request,
        new Response(
          reply.status === 204
            ? null
            : reply.truncated === true
              ? text.slice(0, text.length / 2)
              : text,
          { status: reply.status, headers: { "content-type": "application/json" } },
        ),
      );
    }),
  );

  const layerOf = (tab: ConfirmationGate) =>
    ApiClient.layer.pipe(
      Layer.provide(
        Layer.mergeAll(
          Layer.succeed(HttpClient.HttpClient, http),
          ApiConfig.layer("/api/v1"),
          Layer.succeed(Navigation, {
            location: Effect.succeed("/app/admin/styles"),
            assign: () => Effect.void,
          }),
          tab.layer,
        ),
      ),
    );

  const duplicate = () => {
    const tab = gateOn(copyOf(sessionStorage));

    return { gate: tab, layer: layerOf(tab) };
  };

  return { layer: layerOf(gate), seen, gate, page, duplicate };
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
  localStorage.clear();
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
      expect(gate.snapshot()).toEqual({
        methods: ["password", "totp"],
        writes: 1,
        secrets: 0,
        again: 0,
      });

      gate.confirmed();

      expect(yield* Fiber.join(write)).toEqual({ css: "body { color: red; }" });
      expect(gate.snapshot()).toBeNull();
      expect(writesTo(seen, "/admin/custom_styles").map((entry) => entry.body)).toEqual([
        '{"css":"body { color: red; }"}',
        '{"css":"body { color: red; }"}',
      ]);
    }).pipe(Effect.provide(layer));
  });

  it.live("the two-step setup gate never asks for a confirmation, before or after one", () => {
    const enroll = new TwoFactorRequired({
      message: "Set up two-step sign-in to continue",
      requirement: { kind: "setup", location: "http://campfire.test/two_factor_setup" },
    });

    const gated: Reply = { status: 403, body: encodeErrorBody({ error: enroll }) };
    const reported: unknown[] = [];

    // The first write meets the setup gate; the second is held, then meets it on its replay.
    const { layer, seen, gate } = harness((entry) =>
      seen.length === 2 && entry.path.endsWith("/custom_styles") ? held() : gated,
    );

    const stop = onTwoFactorRequirement((requirement) => reported.push(requirement));

    return Effect.gen(function* () {
      expect((yield* send(styles).pipe(Effect.flip))._tag).toBe("TwoFactorRequired");
      expect(gate.snapshot()).toBeNull();

      const write = yield* Effect.forkChild(send(styles).pipe(Effect.flip));

      yield* untilPrompt(gate);
      gate.confirmed();

      expect((yield* Fiber.join(write))._tag).toBe("TwoFactorRequired");
      expect(gate.snapshot()).toBeNull();
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(3);
      expect(reported).toEqual([enroll.requirement, enroll.requirement]);
    }).pipe(Effect.ensuring(Effect.sync(stop)), Effect.provide(layer));
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

  it.live("leaving a screen cancels the writes it sent; the others still wait", () => {
    const { layer, seen, gate, page } = harness(() => held());

    return Effect.gen(function* () {
      const left = yield* Effect.forkChild(send(styles));

      yield* untilPrompt(gate);
      page.screen = "/app/admin/people";

      const stays = yield* Effect.forkChild(
        send({ ...styles, path: "/admin/workspace/join_code" }),
      );

      yield* untilPrompt(gate, 2);
      gate.screenChanged();

      expect((yield* Fiber.join(left).pipe(Effect.flip))._tag).toBe("ConfirmationCancelled");
      expect(gate.snapshot()?.writes).toBe(1);

      page.screen = "/app/admin";
      gate.screenChanged();

      expect((yield* Fiber.join(stays).pipe(Effect.flip))._tag).toBe("ConfirmationCancelled");
      // Nothing left waiting: the dialog closes, and nothing went again.
      expect(gate.snapshot()).toBeNull();
      expect(seen).toHaveLength(2);
    }).pipe(Effect.provide(layer));
  });

  it.live("a write held after its screen was left fails without asking", () => {
    const asked: number[] = [];

    const { layer, seen, gate, page } = harness(
      () => held(),
      () =>
        Effect.sync(() => {
          page.screen = "/app/admin";
        }),
    );

    gate.subscribe(() => asked.push(gate.snapshot()?.writes ?? 0));

    return Effect.gen(function* () {
      const error = yield* send(styles).pipe(Effect.flip);

      expect(error._tag).toBe("ConfirmationCancelled");
      expect(asked).toEqual([]);
      expect(seen).toHaveLength(1);
    }).pipe(Effect.provide(layer));
  });

  it.live("a write whose screen was left fails even once a confirmation landed elsewhere", () => {
    const latch = Deferred.makeUnsafe<void>();

    const { layer, seen, gate, page } = harness(
      () => held(),
      // The style edit's 403 is slow: the person moves on before it arrives.
      (entry) => (entry.path.endsWith("/custom_styles") ? Deferred.await(latch) : Effect.void),
    );

    return Effect.gen(function* () {
      const write = yield* Effect.forkChild(send(styles));

      for (let turn = 0; turn < 100 && seen.length === 0; turn += 1) {
        yield* Effect.sleep(1);
      }

      page.screen = "/app/admin/people";
      gate.screenChanged();
      // A guarded action on the new page asked and was confirmed.
      gate.confirmed();
      Deferred.doneUnsafe(latch, Effect.void);

      expect((yield* Fiber.join(write).pipe(Effect.flip))._tag).toBe("ConfirmationCancelled");
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(1);
    }).pipe(Effect.provide(layer));
  });
});

const decodeGoogleStart = Schema.decodeUnknownSync(
  Schema.fromJsonString(Schema.Struct({ attempt: Schema.optionalKey(Schema.String) })),
);

/** The fake server's confirmation state, as `require_sudo_mode` and `sudos#continue` keep it. */
interface Server {
  /** Whether the confirmation is fresh. */
  fresh: boolean;
  /** The pending request: the last guarded write it held. */
  pending: SudoRetry | null;
  /** The Google attempt it confirmed, with the pending request it took over. */
  attempt: { readonly id: string; readonly retry: SudoRetry | null } | null;
  /** The signed-in session it names in its replies. */
  session: number;
}

/**
 * What the fake server answers `/sudo/continue` with on its `call`th read; `answer` is what it
 * would answer itself (and computing it does what the server does).
 */
type Continuation = (call: number, answer: () => Reply) => Reply;

/**
 * A server that holds guarded writes until a Google confirmation, unless `refuse`. Starting Google
 * with an attempt confirms it at once (the round trip) and records the attempt, taking over the
 * pending request. The continuation names the attempt it is asked about while it's recorded, and
 * reads it again until acknowledged; a read naming no recorded attempt consumes the pending
 * request, as the classic continuation does.
 */
function googleHarness({
  continuation = (_call, answer) => answer(),
  refuse = false,
  hold,
  locks = "browser",
  respond = () => null,
}: {
  /** What the server answers instead of itself, where this gives a reply. */
  readonly respond?: (entry: Seen, server: Server) => Reply | null;
  readonly continuation?: Continuation;
  readonly refuse?: boolean;
  readonly hold?: (seen: Seen) => Effect.Effect<void>;
  /** Whose Web Locks the tabs use: a working browser's, or the page's own (`navigator.locks`). */
  readonly locks?: "browser" | "page";
} = {}) {
  const server: Server = { fresh: false, pending: null, attempt: null, session: 1 };
  let continues = 0;

  const answer = (entry: Seen): Reply => {
    const asked = entry.query.get("attempt");

    if (!server.fresh) {
      return {
        status: 403,
        body: {
          kind: "ready",
          reauthentication: required().reauthentication,
          session: server.session,
        },
      };
    }

    if (asked !== null && server.attempt?.id === asked) {
      return {
        status: 200,
        body: {
          kind: "confirmed",
          retry: server.attempt.retry,
          attempt: asked,
          session: server.session,
        },
      };
    }

    const retry = server.pending;

    server.pending = null;

    return { status: 200, body: { kind: "confirmed", retry, session: server.session } };
  };

  const built = harness(
    (entry) => {
      const instead = respond(entry, server);

      if (instead !== null) return instead;

      if (entry.path === "/api/v1/sudo/google") {
        server.fresh = !refuse;

        const { attempt } = decodeGoogleStart(entry.body || "{}");

        if (!refuse && attempt !== undefined) {
          server.attempt = { id: attempt, retry: server.pending };
          server.pending = null;
        }

        return {
          status: 200,
          body: { kind: "navigate", location: "https://accounts.test/o", session: server.session },
        };
      }

      if (entry.path === "/api/v1/sudo/continue" && entry.method === "DELETE") {
        if (server.attempt?.id === entry.query.get("attempt")) server.attempt = null;

        return { status: 204, body: null };
      }

      if (entry.path === "/api/v1/sudo/continue") {
        continues += 1;

        return continuation(continues, () => answer(entry));
      }

      if (server.fresh) return { status: 200, body: { saved: entry.path } };

      server.pending = retryOf(entry);

      return held(["password", "google"], entry);
    },
    hold,
    locks === "browser" ? browserLocks() : "page",
  );

  return { ...built, server };
}

/** The attempt the page started Google with. */
const startedAttempt = (seen: readonly Seen[]) =>
  decodeGoogleStart(seen.find((entry) => entry.path === "/api/v1/sudo/google")?.body ?? "{}")
    .attempt;

/** The continuation calls, as method and the attempt they name. */
const continuations = (seen: readonly Seen[]) =>
  seen.flatMap((entry) =>
    entry.path === "/api/v1/sudo/continue" ? [[entry.method, entry.query.get("attempt")]] : [],
  );

const role: ApiRequest = { method: "PATCH", path: "/admin/people/4", body: { role: "member" } };

/** Sends `writes`, waits for them all to be held, and leaves to confirm with Google. */
const leaveForGoogle = (gate: ConfirmationGate, writes: readonly ApiRequest[]) =>
  Effect.gen(function* () {
    for (const write of writes) {
      yield* Effect.forkChild(send(write));
      yield* untilPrompt(gate, writes.indexOf(write) + 1);
    }

    expect((yield* startGoogle(gate, "/app/admin/styles?tab=css")).kind).toBe("navigate");
    expect(sessionStorage.getItem(KEPT_KEY)).toContain("custom_styles");
  });

describe("the Google round trip", () => {
  it.live("a multipart write is never kept for after Google: storage can't hold its body", () => {
    const { layer, gate } = googleHarness();
    const form = new FormData();

    form.append("avatar", new Blob(["png"]), "avatar.png");

    const upload: ApiRequest = { method: "POST", path: "/admin/uploads", form };

    return Effect.gen(function* () {
      const kept = yield* Effect.forkChild(send(styles));

      yield* untilPrompt(gate);

      const multipart = yield* Effect.forkChild(send(upload));

      yield* untilPrompt(gate, 2);
      // The dialog says Google can't bring it back, as for a credential.
      expect(gate.snapshot()).toMatchObject({ writes: 2, secrets: 1 });
      expect((yield* startGoogle(gate, "/app/admin/styles")).kind).toBe("navigate");
      expect((yield* Fiber.join(multipart).pipe(Effect.flip))._tag).toBe("SudoRequired");
      expect(JSON.parse(sessionStorage.getItem(KEPT_KEY) ?? "{}")).toMatchObject({
        writes: [styles],
        dropped: 1,
      });
      yield* Fiber.interrupt(kept);
    }).pipe(Effect.provide(layer));
  });

  it.live("keeps credential-free writes, fails secret ones, and replays the kept once", () => {
    const { layer, seen, gate } = googleHarness();

    return Effect.gen(function* () {
      const kept = yield* Effect.forkChild(send(styles));

      yield* untilPrompt(gate);

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
      // Kept under the attempt the server was given when Google started.
      expect(JSON.parse(stored)).toEqual({
        id: expect.any(String),
        attempt: startedAttempt(seen),
        session: 1,
        user: 7,
        expiresAt: 1_000_000 + 10 * 60_000,
        next: 0,
        returnTo: "/app/admin/styles?tab=css",
        writes: [styles],
        dropped: 1,
        again: 0,
      });

      yield* Fiber.interrupt(kept);

      expect(yield* resumeAfterGoogle(gate)).toEqual({
        confirmed: true,
        returnTo: "/app/admin/styles?tab=css",
        follow: null,
        replayed: 1,
        failure: null,
        dropped: 1,
        again: 0,
        abandoned: 0,
        elsewhere: 0,
        unconfirmed: 0,
        held: null,
        kept: 0,
        unreachable: null,
      });
      expect(sessionStorage.getItem(KEPT_KEY)).toBeNull();
      // The continuation was read for that attempt, then acknowledged once the replay was done.
      expect(continuations(seen)).toEqual([
        ["GET", startedAttempt(seen)],
        ["DELETE", startedAttempt(seen)],
      ]);

      // A reload of the continue page sends nothing again.
      expect((yield* resumeAfterGoogle(gate)).replayed).toBe(0);
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(2);
    }).pipe(Effect.provide(layer));
  });

  it.live("a Google return under someone else replays nothing and forgets the writes", () => {
    const { layer, seen, gate, page } = googleHarness();

    return Effect.gen(function* () {
      yield* leaveForGoogle(gate, [styles]);

      // Signed out and in as someone else in the same tab.
      page.viewer = 8;

      expect(yield* resumeAfterGoogle(gate)).toMatchObject({ confirmed: true, replayed: 0 });
      expect(sessionStorage.getItem(KEPT_KEY)).toBeNull();
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(1);
    }).pipe(Effect.provide(layer));
  });

  it.live("a continuation that names no attempt, or another, keeps the writes unsent", () => {
    // The same pending request each time: only the attempt counts.
    const retry: SudoRetry = {
      method: "PATCH",
      path: "/api/v1/admin/custom_styles",
      returnTo: "/app/admin/styles",
    };

    const bodies = [
      { kind: "confirmed", retry, session: 1 },
      { kind: "confirmed", retry, attempt: "someone-elses-attempt-0001", session: 1 },
    ];

    return Effect.forEach(bodies, (body) => {
      const { layer, seen, gate } = googleHarness({ continuation: () => ({ status: 200, body }) });

      return Effect.gen(function* () {
        yield* leaveForGoogle(gate, [styles]);
        expect(yield* resumeAfterGoogle(gate)).toMatchObject({
          confirmed: true,
          replayed: 0,
          abandoned: 0,
          held: "confirm",
          kept: 1,
        });
        expect(sessionStorage.getItem(KEPT_KEY)).toContain("custom_styles");
        expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(1);
      }).pipe(Effect.provide(layer));
    });
  });

  it.live("an abandoned attempt never takes another tab's confirmation", () => {
    // This tab leaves for Google and gives up there: its attempt is never confirmed.
    const { layer, seen, gate, server } = googleHarness({ refuse: true });

    return Effect.gen(function* () {
      yield* leaveForGoogle(gate, [styles]);

      // Another tab of the same session holds its own style edit and confirms it with Google,
      // and hasn't read its continuation yet.
      const theirs: SudoRetry = {
        method: "PATCH",
        path: "/api/v1/admin/custom_styles",
        returnTo: "/app/admin/styles",
      };

      server.fresh = true;
      server.pending = theirs;
      server.attempt = { id: "the-other-tabs-attempt-01", retry: theirs };

      // This tab follows its continuation link: nothing goes until it confirms for itself.
      expect(yield* resumeAfterGoogle(gate)).toMatchObject({
        confirmed: true,
        replayed: 0,
        held: "confirm",
        kept: 1,
      });
      expect(sessionStorage.getItem(KEPT_KEY)).toContain("custom_styles");
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(1);
      // The other tab's attempt is still there for it.
      expect(server.attempt?.id).toBe("the-other-tabs-attempt-01");
    }).pipe(Effect.provide(layer));
  });

  it.live("an interrupted replay resumes only while the server still confirms its attempt", () =>
    Effect.forEach(["signed in again", "same session"], (session) => {
      const latch = Deferred.makeUnsafe<void>();
      let styleSends = 0;

      const { layer, seen, gate, server } = googleHarness({
        // The first replay never finishes: the page goes (a reload, a closed tab) during it.
        hold: (entry) => {
          if (!entry.path.endsWith("/custom_styles")) return Effect.void;

          styleSends += 1;

          return styleSends === 2 ? Deferred.await(latch) : Effect.void;
        },
      });

      return Effect.gen(function* () {
        yield* leaveForGoogle(gate, [styles, role]);

        const replay = yield* Effect.forkChild(resumeAfterGoogle(gate));

        for (let turn = 0; turn < 100 && styleSends < 2; turn += 1) {
          yield* Effect.sleep(1);
        }

        yield* Fiber.interrupt(replay);
        expect(JSON.parse(sessionStorage.getItem(KEPT_KEY) ?? "{}").writes).toEqual([role]);

        if (session === "signed in again") {
          // Signed out and in again as the same person, who then confirmed something unrelated:
          // the new session never heard of this attempt.
          server.attempt = null;
          server.pending = null;
          server.session = 2;
        }

        const resumed = yield* resumeAfterGoogle(gate);

        expect(sessionStorage.getItem(KEPT_KEY)).toBeNull();

        if (session === "same session") {
          expect(resumed).toMatchObject({ confirmed: true, replayed: 1, abandoned: 0 });
          expect(writesTo(seen, "/admin/people/4")).toHaveLength(2);
          expect(server.attempt).toBeNull();
        } else {
          expect(resumed).toMatchObject({ confirmed: true, replayed: 0, abandoned: 1 });
          expect(writesTo(seen, "/admin/people/4")).toHaveLength(1);
        }
      }).pipe(Effect.provide(layer));
    }),
  );

  it.live("a finished attempt lost to another tab's session write keeps the change", () => {
    const { layer, seen, gate, server } = googleHarness();

    return Effect.gen(function* () {
      yield* leaveForGoogle(gate, [styles]);

      // Another tab's slow 403 answered with the session as it was before Google: its cookie
      // overwrote the finished attempt, and holds that tab's own pending request.
      server.attempt = null;
      server.pending = retryOf({
        method: "PATCH",
        path: "/api/v1/admin/people/4",
        query: new URLSearchParams(),
        body: "",
      });

      expect(yield* resumeAfterGoogle(gate)).toMatchObject({
        confirmed: true,
        replayed: 0,
        abandoned: 0,
        held: "confirm",
        kept: 1,
      });
      expect(sessionStorage.getItem(KEPT_KEY)).toContain("custom_styles");
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(1);

      // Confirm again: a new attempt, and the kept write goes under it, once.
      expect((yield* confirmAgain(gate)).kind).toBe("navigate");
      expect(yield* resumeAfterGoogle(gate)).toMatchObject({
        confirmed: true,
        replayed: 1,
        held: null,
        kept: 0,
      });
      expect(sessionStorage.getItem(KEPT_KEY)).toBeNull();
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(2);
      expect((yield* resumeAfterGoogle(gate)).replayed).toBe(0);
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(2);
    }).pipe(Effect.provide(layer));
  });

  it.live("without Web Locks nothing replays, however often it's tried", () =>
    Effect.forEach(["absent", "refused"], (locks) => {
      const { layer, seen, gate } = googleHarness({ locks: "page" });

      if (locks === "refused") {
        Object.defineProperty(globalThis.navigator, "locks", {
          configurable: true,
          value: { request: () => Promise.reject(new Error("The lock can't be taken.")) },
        });
      }

      return Effect.gen(function* () {
        yield* leaveForGoogle(gate, [styles]);

        for (const _tried of [1, 2]) {
          expect(yield* resumeAfterGoogle(gate)).toMatchObject({
            confirmed: true,
            replayed: 0,
            held: "claim",
            kept: 1,
          });
        }

        expect(sessionStorage.getItem(KEPT_KEY)).toContain("custom_styles");
        expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(1);

        discardKept(gate);
        expect(sessionStorage.getItem(KEPT_KEY)).toBeNull();
      }).pipe(
        Effect.provide(layer),
        Effect.ensuring(Effect.sync(() => Reflect.deleteProperty(globalThis.navigator, "locks"))),
      );
    }),
  );

  it.live("no tab sends a kept write it can't mark as sent for every tab", () => {
    const { layer, seen, gate, page, duplicate } = googleHarness();

    return Effect.gen(function* () {
      yield* leaveForGoogle(gate, [styles]);

      const copy = duplicate();

      // Session storage still works; local storage takes no writes.
      page.markable = false;

      const first = yield* resumeAfterGoogle(gate);

      expect(first).toMatchObject({ confirmed: true, replayed: 0, held: "claim", kept: 1 });

      const second = yield* resumeAfterGoogle(copy.gate).pipe(Effect.provide(copy.layer));

      expect(second).toMatchObject({ replayed: 0, held: "claim", kept: 1 });
      expect(yield* confirmAgain(copy.gate).pipe(Effect.provide(copy.layer))).toMatchObject({
        kind: "navigate",
      });
      expect(yield* resumeAfterGoogle(copy.gate).pipe(Effect.provide(copy.layer))).toMatchObject({
        replayed: 0,
        held: "claim",
        kept: 1,
      });
      // Only the write as first held: the kept copy never went.
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(1);

      // Once writes can be marked again, trying again sends it once, from the tab that confirmed
      // last; the other one's attempt is gone, and it sees the mark.
      page.markable = true;
      expect(yield* resumeAfterGoogle(copy.gate).pipe(Effect.provide(copy.layer))).toMatchObject({
        replayed: 1,
        held: null,
      });
      expect(yield* resumeAfterGoogle(gate)).toMatchObject({
        replayed: 0,
        elsewhere: 1,
        held: null,
      });
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(2);
    }).pipe(Effect.provide(layer));
  });

  it.live("confirming again never outlives the marks of what another tab sent", () => {
    const stalled = Deferred.makeUnsafe<void>();
    let replays = 0;

    const { layer, seen, gate, page, server, duplicate } = googleHarness({
      // The first tab's replay of the first write never returns: it closes there.
      hold: (entry) => {
        if (entry.method === "PATCH" && entry.path === "/api/v1/admin/custom_styles") {
          replays += 1;

          return replays === 2 ? Deferred.await(stalled) : Effect.void;
        }

        return Effect.void;
      },
    });

    return Effect.gen(function* () {
      yield* leaveForGoogle(gate, [styles, role]);

      const copy = duplicate();
      const first = yield* Effect.forkChild(resumeAfterGoogle(gate));

      while (writesTo(seen, "/admin/custom_styles").length < 2) {
        yield* Effect.sleep(1);
      }

      yield* Fiber.interrupt(first);
      // Another tab's session write erased the finished attempt.
      server.attempt = null;

      const inCopy = <A, E>(effect: Effect.Effect<A, E, ApiClient>) =>
        effect.pipe(Effect.provide(copy.layer));

      expect(yield* inCopy(resumeAfterGoogle(copy.gate))).toMatchObject({
        held: "confirm",
        kept: 1,
        // The first tab claimed it but closed before the answer: not sent again, not called sent.
        elsewhere: 0,
        unconfirmed: 1,
      });

      // Confirmed again at minute 9, back at minute 11: past the batch's time limit.
      page.now += 9 * 60_000;
      expect((yield* inCopy(confirmAgain(copy.gate))).kind).toBe("navigate");
      page.now += 2 * 60_000;
      expect((yield* inCopy(confirmAgain(copy.gate))).kind).toBe("error");
      expect(yield* inCopy(resumeAfterGoogle(copy.gate))).toMatchObject({
        replayed: 0,
        held: null,
      });
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(2);
      expect(writesTo(seen, "/admin/people/4")).toHaveLength(1);
      expect(seen.filter((entry) => entry.path === "/api/v1/sudo/google")).toHaveLength(2);
    }).pipe(Effect.provide(layer));
  });

  it.live("a claimed write goes even when its kept copy can't be taken out", () => {
    const { layer, seen, gate, page, duplicate } = googleHarness();

    return Effect.gen(function* () {
      yield* leaveForGoogle(gate, [styles]);

      const copy = duplicate();

      // Session storage reads but takes no writes; the marks still work.
      page.keepable = false;
      expect(yield* resumeAfterGoogle(gate)).toMatchObject({
        confirmed: true,
        replayed: 1,
        failure: null,
        held: null,
      });
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(2);

      // It went from the first tab, so the copy saying so is true.
      page.keepable = true;
      expect(yield* resumeAfterGoogle(copy.gate).pipe(Effect.provide(copy.layer))).toMatchObject({
        replayed: 0,
        elsewhere: 1,
      });
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(2);
    }).pipe(Effect.provide(layer));
  });

  it.live("a batch that runs out while waiting for another tab says its change was lost", () => {
    const stalled = Deferred.makeUnsafe<void>();
    let reads = 0;

    const { layer, seen, gate, page, duplicate } = googleHarness({
      // The first tab's status read hangs, holding the batch's lock.
      hold: (entry) => {
        if (entry.method === "GET" && entry.path === "/api/v1/sudo/continue") {
          reads += 1;

          return reads === 1 ? Deferred.await(stalled) : Effect.void;
        }

        return Effect.void;
      },
    });

    return Effect.gen(function* () {
      yield* leaveForGoogle(gate, [styles]);

      const copy = duplicate();
      const first = yield* Effect.forkChild(resumeAfterGoogle(gate));

      while (reads < 1) {
        yield* Effect.sleep(1);
      }

      const second = yield* Effect.forkChild(
        resumeAfterGoogle(copy.gate).pipe(Effect.provide(copy.layer)),
      );

      yield* Effect.sleep(5);
      page.now += 11 * 60_000;
      Deferred.doneUnsafe(stalled, Effect.void);

      expect((yield* Fiber.join(first)).replayed).toBe(0);
      expect(yield* Fiber.join(second)).toMatchObject({ replayed: 0, abandoned: 1, held: null });
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(1);
    }).pipe(Effect.provide(layer));
  });

  it.live("discarding while another tab holds the lock stops waiting and sends nothing", () => {
    const stalled = Deferred.makeUnsafe<void>();
    let reads = 0;

    const { layer, seen, gate, duplicate } = googleHarness({
      hold: (entry) => {
        if (entry.method === "GET" && entry.path === "/api/v1/sudo/continue") {
          reads += 1;

          return reads === 1 ? Deferred.await(stalled) : Effect.void;
        }

        return Effect.void;
      },
    });

    return Effect.gen(function* () {
      yield* leaveForGoogle(gate, [styles]);

      const copy = duplicate();
      const first = yield* Effect.forkChild(resumeAfterGoogle(gate));

      while (reads < 1) {
        yield* Effect.sleep(1);
      }

      const leaving = new AbortController();

      const second = yield* Effect.forkChild(
        resumeAfterGoogle(copy.gate, { signal: leaving.signal }).pipe(Effect.provide(copy.layer)),
      );

      yield* Effect.sleep(5);
      discardKept(copy.gate);
      leaving.abort();

      // The copy is done before the first tab lets go of the lock, having asked nothing.
      const discarded = yield* Fiber.join(second).pipe(Effect.timeoutOption(200));

      expect(Option.getOrNull(discarded)).toMatchObject({ replayed: 0, held: null });
      expect(reads).toBe(1);

      Deferred.doneUnsafe(stalled, Effect.void);
      expect((yield* Fiber.join(first)).replayed).toBe(1);
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(2);
    }).pipe(Effect.provide(layer));
  });

  it.live("a write discarded during its token refresh is never sent again", () => {
    const booted = Deferred.makeUnsafe<void>();
    let boots = 0;

    const { layer, seen, gate } = googleHarness({
      // The replay's token is stale; the fresh one is slow to come.
      respond: (entry, server) => {
        if (entry.path === "/api/v1/boot") return { status: 200, body: { csrfToken: "fresh" } };

        return server.fresh && writesTo(seen, "/admin/custom_styles").length === 2
          ? { status: 422, body: null }
          : null;
      },
      hold: (entry) => {
        if (entry.path !== "/api/v1/boot") return Effect.void;

        boots += 1;

        return Deferred.await(booted);
      },
    });

    return Effect.gen(function* () {
      yield* leaveForGoogle(gate, [styles]);

      const leaving = new AbortController();
      const replay = yield* Effect.forkChild(resumeAfterGoogle(gate, { signal: leaving.signal }));

      while (boots < 1) {
        yield* Effect.sleep(1);
      }

      discardKept(gate);
      leaving.abort();
      Deferred.doneUnsafe(booted, Effect.void);
      expect((yield* Fiber.join(replay)).replayed).toBe(0);
      yield* Effect.sleep(5);
      // The held write and the stale replay: nothing went after Discard.
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(2);
    }).pipe(Effect.provide(layer));
  });

  it.live("a write another tab claimed but never finished isn't reported as sent", () => {
    const stalled = Deferred.makeUnsafe<void>();

    const { layer, seen, gate, duplicate } = googleHarness({
      // The first tab's replay never answers: it closes there.
      hold: (entry) =>
        entry.method === "PATCH" && writesTo(seen, "/admin/custom_styles").length === 2
          ? Deferred.await(stalled)
          : Effect.void,
    });

    return Effect.gen(function* () {
      yield* leaveForGoogle(gate, [styles]);

      const copy = duplicate();
      const first = yield* Effect.forkChild(resumeAfterGoogle(gate));

      while (writesTo(seen, "/admin/custom_styles").length < 2) {
        yield* Effect.sleep(1);
      }

      yield* Fiber.interrupt(first);
      expect(yield* resumeAfterGoogle(copy.gate).pipe(Effect.provide(copy.layer))).toMatchObject({
        confirmed: true,
        replayed: 0,
        elsewhere: 0,
        unconfirmed: 1,
      });
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(2);
    }).pipe(Effect.provide(layer));
  });

  it.live("a change lost before a failed status read is still reported after it", () => {
    const { layer, seen, gate, page } = googleHarness({
      continuation: (call, answer) => (call === 1 ? { status: 503, body: {} } : answer()),
    });

    return Effect.gen(function* () {
      yield* leaveForGoogle(gate, [styles]);
      page.now += 11 * 60_000;

      const first = yield* resumeAfterGoogle(gate);

      expect(first.unreachable).not.toBeNull();
      expect(yield* resumeAfterGoogle(gate)).toMatchObject({
        confirmed: true,
        replayed: 0,
        abandoned: 1,
      });
      // Reported once.
      expect((yield* resumeAfterGoogle(gate)).abandoned).toBe(0);
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(1);
    }).pipe(Effect.provide(layer));
  });

  it.live("a status read whose body is lost reads the same answer again", () => {
    const { layer, seen, gate, server } = googleHarness({
      // The server answers (and its session cookie arrives), but the body is cut short.
      continuation: (call, answer) => (call === 1 ? { ...answer(), truncated: true } : answer()),
    });

    return Effect.gen(function* () {
      yield* leaveForGoogle(gate, [styles]);

      const first = yield* resumeAfterGoogle(gate);

      expect(first).toMatchObject({ confirmed: false, replayed: 0 });
      expect(first.unreachable).not.toBeNull();
      expect(sessionStorage.getItem(KEPT_KEY)).toContain("custom_styles");

      // Try again.
      expect(yield* resumeAfterGoogle(gate)).toMatchObject({
        confirmed: true,
        replayed: 1,
        abandoned: 0,
      });
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(2);
      expect(server.attempt).toBeNull();
    }).pipe(Effect.provide(layer));
  });

  it.live("a hand-off older than its time limit replays nothing and says so", () => {
    const { layer, seen, gate, page } = googleHarness();

    return Effect.gen(function* () {
      yield* leaveForGoogle(gate, [styles]);
      page.now += 11 * 60_000;

      expect(yield* resumeAfterGoogle(gate)).toMatchObject({
        confirmed: true,
        replayed: 0,
        abandoned: 1,
      });

      expect(sessionStorage.getItem(KEPT_KEY)).toBeNull();
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(1);
    }).pipe(Effect.provide(layer));
  });

  it.live("a failed status read keeps the writes for another try", () => {
    const { layer, seen, gate } = googleHarness({
      continuation: (call, answer) => (call === 1 ? { status: 500, body: {} } : answer()),
    });

    return Effect.gen(function* () {
      yield* leaveForGoogle(gate, [styles]);

      const first = yield* resumeAfterGoogle(gate);

      expect(first).toMatchObject({ confirmed: false, replayed: 0 });
      expect(first.unreachable).not.toBeNull();
      expect(sessionStorage.getItem(KEPT_KEY)).toContain("custom_styles");
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(1);

      expect(yield* resumeAfterGoogle(gate)).toMatchObject({ confirmed: true, replayed: 1 });
      expect(sessionStorage.getItem(KEPT_KEY)).toBeNull();
    }).pipe(Effect.provide(layer));
  });

  it.live("each kept write leaves storage as its replay begins", () => {
    const storedAtReplay: (string | null)[] = [];

    const { layer, gate } = googleHarness({
      hold: (entry) =>
        Effect.sync(() => {
          if (sessionStorage.getItem(KEPT_KEY) !== null || storedAtReplay.length > 0) {
            if (entry.path === "/api/v1/sudo/continue" || entry.path === "/api/v1/sudo/google") {
              return;
            }

            storedAtReplay.push(sessionStorage.getItem(KEPT_KEY));
          }
        }),
    });

    return Effect.gen(function* () {
      yield* leaveForGoogle(gate, [styles, role]);
      expect(yield* resumeAfterGoogle(gate)).toMatchObject({ confirmed: true, replayed: 2 });

      const [atFirst, atSecond] = storedAtReplay;

      expect(JSON.parse(atFirst ?? "{}").writes).toEqual([role]);
      expect(atSecond).toBeNull();
    }).pipe(Effect.provide(layer));
  });

  it.live("a duplicated tab never sends the writes another tab is replaying", () => {
    const acknowledged = Deferred.makeUnsafe<void>();

    const { layer, seen, gate, duplicate } = googleHarness({
      // The first tab's acknowledgement is slow: both tabs read the confirmed attempt first.
      hold: (entry) =>
        entry.method === "DELETE" && entry.path === "/api/v1/sudo/continue"
          ? Deferred.await(acknowledged)
          : Effect.void,
    });

    return Effect.gen(function* () {
      yield* leaveForGoogle(gate, [styles, role]);

      // Duplicated before the continue page loaded: the copy holds the same two writes.
      const copy = duplicate();
      const first = yield* Effect.forkChild(resumeAfterGoogle(gate));

      const second = yield* Effect.forkChild(
        resumeAfterGoogle(copy.gate).pipe(Effect.provide(copy.layer)),
      );

      // Both continue pages are open before anything is acknowledged.
      yield* Effect.sleep(10);
      Deferred.doneUnsafe(acknowledged, Effect.void);

      const results = [yield* Fiber.join(first), yield* Fiber.join(second)];

      expect(
        results.map((result) => [result.replayed, result.elsewhere, result.abandoned]),
      ).toEqual([
        [2, 0, 0],
        [0, 2, 0],
      ]);
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(2);
      expect(writesTo(seen, "/admin/people/4")).toHaveLength(2);
    }).pipe(Effect.provide(layer));
  });

  it.live("a duplicated tab opened after the replay says another tab sent it", () => {
    const { layer, seen, gate, duplicate } = googleHarness();

    return Effect.gen(function* () {
      yield* leaveForGoogle(gate, [styles]);

      const copy = duplicate();

      expect(yield* resumeAfterGoogle(gate)).toMatchObject({
        replayed: 1,
        elsewhere: 0,
      });
      // The first tab acknowledged the attempt: the server no longer names it.
      expect(yield* resumeAfterGoogle(copy.gate).pipe(Effect.provide(copy.layer))).toMatchObject({
        confirmed: true,
        replayed: 0,
        elsewhere: 1,
        abandoned: 0,
      });
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(2);
    }).pipe(Effect.provide(layer));
  });

  it.live("a refused Google return keeps the writes and asks to confirm again", () => {
    const { layer, seen, gate } = googleHarness({ refuse: true });

    return Effect.gen(function* () {
      yield* leaveForGoogle(gate, [styles]);

      expect(yield* resumeAfterGoogle(gate)).toEqual({
        confirmed: false,
        returnTo: "/app/admin/styles?tab=css",
        follow: null,
        replayed: 0,
        failure: null,
        dropped: 0,
        again: 0,
        abandoned: 0,
        elsewhere: 0,
        unconfirmed: 0,
        held: "confirm",
        kept: 1,
        unreachable: null,
      });
      expect(sessionStorage.getItem(KEPT_KEY)).toContain("custom_styles");
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(1);

      // The person gives up on it.
      discardKept(gate);
      expect(sessionStorage.getItem(KEPT_KEY)).toBeNull();
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
    expect(spaReturnPath("/app/sudo/new")).toBe("/app/");
    expect(spaReturnPath(null)).toBe("/app/");
  });
});

/** The server is down. */
const down: Reply = { status: 503, body: "Service Unavailable" };

/** What the classic gate held for a page asked for in full: a download, say. */
const exportRetry: SudoRetry = {
  method: "GET",
  path: "/account/audit_log.csv?action=sign_in",
  returnTo: "/app/settings/audit-log",
};

/** What it held for a form the person sent in full: connecting Google, say. */
const connectRetry: SudoRetry = {
  method: "POST",
  path: "/google/connect",
  returnTo: "/app/settings/integrations",
};

/** The password check at `POST /sudo`, which the Google harness otherwise holds as a write. */
const passwordCheck = (entry: Seen, server: Server): Reply | null => {
  if (entry.path !== "/api/v1/sudo" || entry.method !== "POST") return null;

  if (entry.body.includes("wrong")) return refused;

  server.fresh = true;

  const retry = server.pending;

  server.pending = null;

  return { status: 200, body: { kind: "confirmed", retry, session: server.session } };
};

describe("a full-page action checked first", () => {
  it.live("goes at once while the confirmation is fresh, without a prompt", () => {
    const { layer, seen, gate, server } = googleHarness();

    server.fresh = true;

    return Effect.gen(function* () {
      expect(yield* confirmFirst(gate)).toEqual({ kind: "go" });
      expect(gate.snapshot()).toBeNull();
      expect(continuations(seen)).toEqual([["GET", null]]);
    }).pipe(Effect.provide(layer));
  });

  it.live("asks in the dialog when it isn't, and goes once the person confirms", () => {
    const { layer, gate } = googleHarness({ respond: passwordCheck });

    return Effect.gen(function* () {
      const check = yield* Effect.forkChild(confirmFirst(gate));

      yield* untilPrompt(gate);
      expect(gate.snapshot()).toEqual({
        methods: ["password", "totp"],
        writes: 1,
        secrets: 0,
        again: 1,
      });
      expect(yield* submit(gate, { kind: "password", password: "wrong" })).toBe(
        "Confirmation failed. Try again.",
      );
      expect(gate.snapshot()?.again).toBe(1);
      expect(yield* submit(gate, { kind: "password", password: "right" })).toBeNull();
      expect(yield* Fiber.join(check)).toEqual({ kind: "go" });
      expect(gate.snapshot()).toBeNull();
    }).pipe(Effect.provide(layer));
  });

  it.live("stays when the person cancels, or the screen changes", () => {
    const { layer, gate, page } = googleHarness();

    return Effect.gen(function* () {
      const cancelled = yield* Effect.forkChild(confirmFirst(gate));

      yield* untilPrompt(gate);
      gate.cancel();
      expect(yield* Fiber.join(cancelled)).toEqual({ kind: "stay" });

      const left = yield* Effect.forkChild(confirmFirst(gate));

      yield* untilPrompt(gate);
      page.screen = "/app/r/1";
      gate.screenChanged();
      expect(yield* Fiber.join(left)).toEqual({ kind: "stay" });
      expect(gate.snapshot()).toBeNull();
    }).pipe(Effect.provide(layer));
  });

  it.live("says why when the server can't be asked", () => {
    const { layer, gate } = googleHarness({
      respond: (entry) => (entry.path === "/api/v1/sudo/continue" ? down : null),
    });

    return Effect.gen(function* () {
      const outcome = yield* confirmFirst(gate);

      expect(outcome.kind).toBe("unavailable");
      expect(gate.snapshot()).toBeNull();
    }).pipe(Effect.provide(layer));
  });

  it.live("leaving for Google, it stays: the return asks the person to click again", () => {
    const { layer, seen, gate } = googleHarness();

    return Effect.gen(function* () {
      const kept = yield* Effect.forkChild(send(styles));

      yield* untilPrompt(gate);

      const check = yield* Effect.forkChild(confirmFirst(gate));

      yield* untilPrompt(gate, 2);
      expect(gate.snapshot()).toMatchObject({ writes: 2, secrets: 0, again: 1 });
      expect((yield* startGoogle(gate, "/app/admin/styles")).kind).toBe("navigate");
      expect(yield* Fiber.join(check)).toEqual({ kind: "stay" });
      expect(JSON.parse(sessionStorage.getItem(KEPT_KEY) ?? "{}")).toMatchObject({
        writes: [styles],
        dropped: 0,
        again: 1,
      });
      yield* Fiber.interrupt(kept);

      expect(yield* resumeAfterGoogle(gate)).toMatchObject({
        confirmed: true,
        returnTo: "/app/admin/styles",
        follow: null,
        replayed: 1,
        again: 1,
      });
      expect(writesTo(seen, "/admin/custom_styles")).toHaveLength(2);
    }).pipe(Effect.provide(layer));
  });

  it.live("with only the action waiting, the return still says to click again", () => {
    const { layer, gate } = googleHarness();

    return Effect.gen(function* () {
      const check = yield* Effect.forkChild(confirmFirst(gate));

      yield* untilPrompt(gate);
      expect((yield* startGoogle(gate, "/app/settings/integrations")).kind).toBe("navigate");
      expect(yield* Fiber.join(check)).toEqual({ kind: "stay" });

      expect(yield* resumeAfterGoogle(gate)).toEqual({
        confirmed: true,
        returnTo: "/app/settings/integrations",
        follow: null,
        replayed: 0,
        failure: null,
        dropped: 0,
        again: 1,
        abandoned: 0,
        elsewhere: 0,
        unconfirmed: 0,
        held: null,
        kept: 0,
        unreachable: null,
      });
      expect(sessionStorage.getItem(KEPT_KEY)).toBeNull();
    }).pipe(Effect.provide(layer));
  });
});

describe("/app/sudo/new", () => {
  it.live("reads whether the confirmation is fresh, and what the server held", () => {
    const { layer, gate, server } = googleHarness();

    server.pending = exportRetry;

    return Effect.gen(function* () {
      expect(yield* sudoStatus()).toMatchObject({ kind: "ready" });

      server.fresh = true;
      expect(yield* sudoStatus()).toMatchObject({ kind: "confirmed", retry: exportRetry });
      // Consumed: the next read has nothing held.
      expect(yield* sudoStatus()).toMatchObject({ kind: "confirmed", retry: null });
      expect(gate.snapshot()).toBeNull();
    }).pipe(Effect.provide(layer));
  });

  it.live("says the server couldn't be asked rather than failing", () => {
    const { layer } = googleHarness({
      respond: () => down,
    });

    return Effect.gen(function* () {
      expect((yield* sudoStatus()).kind).toBe("unreachable");
    }).pipe(Effect.provide(layer));
  });

  it.live("a password confirms here and hands back what the server held", () => {
    const { layer, gate, server } = googleHarness({ respond: passwordCheck });

    server.pending = connectRetry;

    return Effect.gen(function* () {
      expect(yield* confirmHere(gate, { kind: "password", password: "wrong" })).toEqual({
        kind: "refused",
        message: "Confirmation failed. Try again.",
      });
      expect(yield* confirmHere(gate, { kind: "password", password: "right" })).toEqual({
        kind: "confirmed",
        retry: connectRetry,
      });
    }).pipe(Effect.provide(layer));
  });

  it.live("Google starts with no attempt and keeps nothing", () => {
    const { layer, seen, server } = googleHarness();

    server.pending = exportRetry;

    return Effect.gen(function* () {
      expect(yield* googleHere()).toEqual({
        kind: "navigate",
        location: "https://accounts.test/o",
      });
      expect(seen.find((entry) => entry.path === "/api/v1/sudo/google")?.body).toBe("{}");
      expect(sessionStorage.getItem(KEPT_KEY)).toBeNull();
    }).pipe(Effect.provide(layer));
  });

  it.live("back from Google with nothing kept, a page asked for loads again", () => {
    const { layer, gate, server } = googleHarness();

    server.pending = exportRetry;

    return Effect.gen(function* () {
      yield* googleHere();

      expect(yield* resumeAfterGoogle(gate)).toMatchObject({
        confirmed: true,
        returnTo: "/app/settings/audit-log",
        follow: "/account/audit_log.csv?action=sign_in",
        again: 0,
      });
    }).pipe(Effect.provide(layer));
  });

  it.live("back from Google with nothing kept, a form goes back to be sent again", () => {
    const { layer, gate, server } = googleHarness();

    server.pending = connectRetry;

    return Effect.gen(function* () {
      yield* googleHere();

      expect(yield* resumeAfterGoogle(gate)).toMatchObject({
        confirmed: true,
        returnTo: "/app/settings/integrations",
        follow: null,
      });
    }).pipe(Effect.provide(layer));
  });

  it.live("back from a refused Google with something held, it asks again on the page", () => {
    const { layer, gate, server } = googleHarness({
      refuse: true,
      respond: (entry, held) =>
        entry.path === "/api/v1/sudo/continue" && !held.fresh
          ? {
              status: 403,
              body: {
                kind: "ready",
                reauthentication: { methods: ["password"], retry: held.pending },
                session: held.session,
              },
            }
          : null,
    });

    server.pending = connectRetry;

    return Effect.gen(function* () {
      yield* googleHere();

      expect(yield* resumeAfterGoogle(gate)).toMatchObject({
        confirmed: false,
        returnTo: "/app/sudo/new",
        follow: null,
      });
    }).pipe(Effect.provide(layer));
  });

  it("follows only pages of this site", () => {
    const go = (path: string, method = "GET") =>
      afterConfirming({ kind: "confirmed", retry: { method, path, returnTo: "/app/x" } }, "/app/")
        .follow;

    expect(go("/slack/oauth/start?return_to=x")).toBe("/slack/oauth/start?return_to=x");
    expect(go("/api/v1/admin/people")).toBeNull();
    expect(go("//evil.test/")).toBeNull();
    expect(go("https://evil.test/")).toBeNull();
    expect(go("/google/connect", "POST")).toBeNull();
    expect(afterConfirming({ kind: "confirmed", retry: null }, "/app/r/1")).toEqual({
      returnTo: "/app/r/1",
      follow: null,
    });
  });
});
