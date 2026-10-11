import { afterEach, describe, expect, it } from "@effect/vitest";
import { Effect, Layer, Schema } from "effect";
import * as HttpClient from "effect/http/HttpClient";
import * as HttpClientError from "effect/http/HttpClientError";
import * as HttpClientRequest from "effect/http/HttpClientRequest";
import * as HttpClientResponse from "effect/http/HttpClientResponse";
import { passwordSignIn, submitChallenge } from "./auth-endpoints.ts";
import { readBoot } from "./boot.ts";
import { ApiClient, ApiConfig, Navigation } from "./client.ts";
import { markRead, me, messages, room } from "./endpoints.ts";
import {
  type ApiError,
  ApiErrorResponse,
  InvalidAuthenticityToken,
  NetworkError,
  NotFound,
  RateLimited,
  ServerError,
  SudoRequired,
  TwoFactorRequired,
  Unauthorized,
} from "./errors.ts";
import { resumeSudo, submitSudo } from "./sudo-endpoints.ts";
import { meFixture, roomDetailFixture } from "./testing.ts";
import { completeTour } from "./tour-endpoints.ts";
import { readTwoFactorSetup, submitTwoFactorSetup } from "./two-factor-setup-endpoints.ts";

/** What the fake server saw. */
interface Seen {
  readonly method: string;
  readonly url: URL;
  readonly csrf: string | null;
  readonly body: string;
}

/** A canned reply: a status and body, or a dropped connection. */
type Reply = { readonly status: number; readonly body: string } | "network";

const json = <Body>(status: number, body: Body): Reply => ({ status, body: JSON.stringify(body) });

const encodeErrorBody = Schema.encodeSync(ApiErrorResponse);

/** A typed error reply, encoded the way the server sends it. */
const errorReply = (status: number, error: ApiError): Reply =>
  json(status, encodeErrorBody({ error }));

const bootJson = {
  user: { id: 7, name: "Ada Lovelace", avatarUrl: "/users/7/avatar" },
  account: {
    name: "Smart Data",
    logoUrl: null,
    logoStillUrl: null,
    bannerUrl: null,
    bannerStillUrl: null,
    uploadLimitBytes: 100 * 1024 * 1024,
  },
  theme: "dark",
  textSize: "default",
  customStyles: null,
  cableUrl: "/cable",
  serviceWorkerUrl: null,
  version: "2.0.0",
  revision: null,
  appearancePreferences: null,
};

/**
 * An `ApiClient` over a fake `HttpClient` whose replies come from `answer` (by request, in
 * order). Returns the layer, what the server saw, and where Navigation was sent.
 */
function harness(answer: (seen: Seen, index: number) => Reply) {
  const seen: Seen[] = [];
  const navigations: string[] = [];

  const http = HttpClient.make((request, url) =>
    Effect.gen(function* () {
      const web = yield* HttpClientRequest.toWeb(request).pipe(Effect.orDie);
      const body = yield* Effect.promise(() => web.text());

      const entry: Seen = {
        method: request.method,
        url,
        csrf: request.headers["x-csrf-token"] ?? null,
        body,
      };

      seen.push(entry);

      const reply = answer(entry, seen.length - 1);

      if (reply === "network") {
        return yield* new HttpClientError.HttpClientError({
          reason: new HttpClientError.TransportError({ request, description: "offline" }),
        });
      }

      return HttpClientResponse.fromWeb(
        request,
        // A 204 can't carry a body, not even an empty one.
        new Response(reply.status === 204 ? null : reply.body, {
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
          location: Effect.succeed("/app/rooms/12?message=4"),
          assign: (url: string) => Effect.sync(() => navigations.push(url)),
        }),
      ),
    ),
  );

  return { layer, seen, navigations };
}

function setCsrfMeta(token: string) {
  const meta = document.createElement("meta");

  meta.name = "csrf-token";
  meta.content = token;
  document.head.append(meta);
}

afterEach(() => {
  for (const node of document.querySelectorAll('meta[name="csrf-token"], #boot')) {
    node.remove();
  }
});

describe("two-factor setup HTTP statuses", () => {
  for (const status of [422, 429]) {
    it.effect(`returns the setup and error wording on ${status} without navigating`, () => {
      const body = {
        kind: "error",
        message:
          status === 422
            ? "That code didn't work. Check your authenticator app and try again."
            : "Too many attempts. Try again in a few minutes.",
        setup: {
          secret: "ABCD1234",
          manualKey: "ABCD 1234",
          otpauthUri: "otpauth://totp/Smartfire:ada?secret=ABCD1234",
          qrSvg: "<svg/>",
        },
      };

      const { layer, navigations, seen } = harness(() => json(status, body));

      return Effect.gen(function* () {
        const client = yield* ApiClient;

        yield* client.setCsrfToken("held-csrf");

        expect(yield* submitTwoFactorSetup({ code: "123 456" })).toEqual(body);

        expect(seen).toMatchObject([
          { method: "POST", csrf: "held-csrf", body: '{"code":"123 456"}' },
        ]);
        expect(navigations).toEqual([]);
      }).pipe(Effect.provide(layer));
    });
  }

  it.effect("navigates a signed-out setup request on 401", () => {
    const body = { kind: "navigate", location: "http://campfire.test/session/new" };
    const { layer, navigations } = harness(() => json(401, body));

    return Effect.gen(function* () {
      expect(yield* readTwoFactorSetup()).toEqual(body);
      expect(navigations).toEqual([body.location]);
    }).pipe(Effect.provide(layer));
  });
});

describe("two-factor enrollment gate", () => {
  it.effect("decodes a required setup without redirecting or exposing provisioning secrets", () => {
    const expected = new TwoFactorRequired({
      message: "Set up two-step sign-in to continue",
      requirement: { kind: "setup", location: "http://campfire.test/two_factor_setup" },
    });

    const { layer, navigations, seen } = harness(() => errorReply(403, expected));

    return Effect.gen(function* () {
      const client = yield* ApiClient;

      const error = yield* client
        .execute({ method: "GET", path: "/settings" }, Schema.decodeUnknownEffect(Schema.String))
        .pipe(Effect.flip);

      expect(error._tag).toBe("TwoFactorRequired");
      expect(error).toMatchObject({ requirement: expected.requirement });
      expect(navigations).toEqual([]);
      expect(seen).toHaveLength(1);
    }).pipe(Effect.provide(layer));
  });
});

describe("sudo HTTP statuses", () => {
  it.effect("requires sign-in for a sudo 401 that cannot decode", () => {
    const { layer, navigations } = harness(() => json(401, { kind: "error", fieldErrors: {} }));

    return Effect.gen(function* () {
      expect((yield* resumeSudo().pipe(Effect.flip))._tag).toBe("Unauthorized");
      expect(navigations).toEqual(["/session/new?return_to=%2Fapp%2Frooms%2F12%3Fmessage%3D4"]);
    }).pipe(Effect.provide(layer));
  });

  it.effect("navigates to sign-in when a sudo before-action refuses the session", () => {
    const next = { kind: "navigate", location: "http://campfire.test/session/new" };
    const { layer, navigations } = harness(() => json(401, next));

    return Effect.gen(function* () {
      expect(yield* resumeSudo()).toEqual(next);
      expect(navigations).toEqual(["http://campfire.test/session/new"]);
    }).pipe(Effect.provide(layer));
  });

  it.effect("decodes credential failures and expiry without leaving the SPA", () => {
    setCsrfMeta("held-token");

    const refused = { kind: "error", message: "Confirmation failed. Try again." };
    const expired = { kind: "ready", reauthentication: { methods: ["password"], retry: null } };

    const { layer, navigations } = harness((_, index) =>
      json(index === 0 ? 401 : 403, index === 0 ? refused : expired),
    );

    return Effect.gen(function* () {
      expect(yield* submitSudo({ kind: "password", password: "wrong" })).toEqual(refused);
      expect(yield* resumeSudo()).toEqual(expired);
      expect(navigations).toEqual([]);
    }).pipe(Effect.provide(layer));
  });

  it.effect("retains available methods and retry metadata on protected writes", () => {
    const error = new SudoRequired({
      message: "Confirm your password to continue",
      reauthentication: {
        methods: ["password", "totp"],
        retry: { method: "PATCH", path: "/api/v1/admin/custom_styles", returnTo: "/app/admin" },
      },
    });

    const { layer, navigations } = harness(() => errorReply(403, error));

    return Effect.gen(function* () {
      expect(yield* me().pipe(Effect.flip)).toEqual(error);
      expect(navigations).toEqual([]);
    }).pipe(Effect.provide(layer));
  });
});

describe("ApiClient", () => {
  it.effect("sends the meta CSRF token on writes, not on reads", () => {
    setCsrfMeta("meta-token");

    const { layer, seen } = harness((request) =>
      request.method === "GET"
        ? json(200, meFixture)
        : json(200, { roomId: 12, unread: false, firstUnreadMessageId: null, unreadCount: 0 }),
    );

    return Effect.gen(function* () {
      yield* me();
      yield* markRead(12);

      expect(seen.map((request) => [request.method, request.url.pathname, request.csrf])).toEqual([
        ["GET", "/api/v1/me", null],
        ["POST", "/api/v1/rooms/12/read", "meta-token"],
      ]);
    }).pipe(Effect.provide(layer));
  });

  it.effect("returns the wire JSON unchanged (strings, not DateTime or branded values)", () => {
    const { layer } = harness(() => json(200, meFixture));

    return Effect.gen(function* () {
      expect(yield* me()).toEqual(meFixture);
    }).pipe(Effect.provide(layer));
  });

  it.effect("puts the page cursor in the query", () => {
    const { layer, seen } = harness(() =>
      json(200, { messages: [], users: [], before: null, after: null, saved: [] }),
    );

    return Effect.gen(function* () {
      yield* messages(12, { around: 40 });
      yield* messages(12, null);

      expect(seen.map((request) => request.url.search)).toEqual(["?around=40", ""]);
    }).pipe(Effect.provide(layer));
  });

  it.effect("on a stale-token 422, refreshes from /boot and retries once", () => {
    setCsrfMeta("old-token");

    const { layer, seen } = harness((request, index) => {
      if (request.url.pathname === "/api/v1/boot") {
        return json(200, { ...bootJson, csrfToken: "new-token" });
      }

      return index === 0
        ? errorReply(422, new InvalidAuthenticityToken({ message: "stale" }))
        : json(200, { roomId: 12, unread: false, firstUnreadMessageId: null, unreadCount: 0 });
    });

    return Effect.gen(function* () {
      expect(yield* markRead(12)).toEqual({
        roomId: 12,
        unread: false,
        firstUnreadMessageId: null,
        unreadCount: 0,
      });
      expect(seen.map((request) => [request.url.pathname, request.csrf])).toEqual([
        ["/api/v1/rooms/12/read", "old-token"],
        ["/api/v1/boot", null],
        ["/api/v1/rooms/12/read", "new-token"],
      ]);
    }).pipe(Effect.provide(layer));
  });

  it.effect("retries a stale token only once", () => {
    setCsrfMeta("old-token");

    const { layer, seen } = harness((request) =>
      request.url.pathname === "/api/v1/boot"
        ? json(200, { ...bootJson, csrfToken: "new-token" })
        : errorReply(422, new InvalidAuthenticityToken({ message: "stale" })),
    );

    return Effect.gen(function* () {
      expect(yield* Effect.flip(markRead(12))).toEqual(
        new InvalidAuthenticityToken({ message: "stale" }),
      );
      expect(seen.map((request) => request.url.pathname)).toEqual([
        "/api/v1/rooms/12/read",
        "/api/v1/boot",
        "/api/v1/rooms/12/read",
      ]);
    }).pipe(Effect.provide(layer));
  });

  it.effect("treats a 422 without a JSON body as a stale token too", () => {
    setCsrfMeta("old-token");

    const { layer, seen } = harness((request, index) => {
      if (request.url.pathname === "/api/v1/boot") {
        return json(200, { ...bootJson, csrfToken: "new-token" });
      }

      return index === 0
        ? { status: 422, body: "Unprocessable" }
        : json(200, { roomId: 12, unread: false, firstUnreadMessageId: null, unreadCount: 0 });
    });

    return Effect.gen(function* () {
      yield* markRead(12);

      expect(seen.at(-1)?.csrf).toBe("new-token");
    }).pipe(Effect.provide(layer));
  });

  it.effect("sends a root request from the origin, retrying its bare stale-token 422", () => {
    setCsrfMeta("old-token");

    const { layer, seen } = harness((request, index) => {
      if (request.url.pathname === "/api/v1/boot") {
        return json(200, { ...bootJson, csrfToken: "new-token" });
      }

      return index === 0 ? { status: 422, body: "" } : { status: 204, body: "" };
    });

    return Effect.gen(function* () {
      yield* completeTour();

      expect(seen.map((request) => [request.method, request.url.pathname, request.csrf])).toEqual([
        ["PATCH", "/users/me/tour", "old-token"],
        ["GET", "/api/v1/boot", null],
        ["PATCH", "/users/me/tour", "new-token"],
      ]);
    }).pipe(Effect.provide(layer));
  });

  it.effect("without a meta token (dev), fetches one before the first write", () => {
    const { layer, seen } = harness((request) =>
      request.url.pathname === "/api/v1/boot"
        ? json(200, { ...bootJson, csrfToken: "dev-token" })
        : json(200, { roomId: 12, unread: false, firstUnreadMessageId: null, unreadCount: 0 }),
    );

    return Effect.gen(function* () {
      yield* markRead(12);
      yield* markRead(12);

      expect(seen.map((request) => [request.url.pathname, request.csrf])).toEqual([
        ["/api/v1/boot", null],
        ["/api/v1/rooms/12/read", "dev-token"],
        ["/api/v1/rooms/12/read", "dev-token"],
      ]);
    }).pipe(Effect.provide(layer));
  });

  it.effect("on 401, sends the person to sign in and back, and fails Unauthorized", () => {
    const { layer, navigations } = harness(() =>
      errorReply(401, new Unauthorized({ message: "Signed out" })),
    );

    return Effect.gen(function* () {
      expect(yield* Effect.flip(me())).toEqual(new Unauthorized({ message: "Signed out" }));
      expect(navigations).toEqual(["/session/new?return_to=%2Fapp%2Frooms%2F12%3Fmessage%3D4"]);
    }).pipe(Effect.provide(layer));
  });

  it.effect("decodes typed error bodies", () => {
    const replies: Reply[] = [
      errorReply(404, new NotFound({ message: "No such room" })),
      errorReply(429, new RateLimited({ message: "Slow down", retryAfter: 30 })),
    ];

    const { layer } = harness((_request, index) => replies[index] ?? "network");

    return Effect.gen(function* () {
      const notFound = yield* Effect.flip(room(99));

      expect(notFound).toBeInstanceOf(NotFound);
      expect(notFound).toEqual(new NotFound({ message: "No such room" }));
      expect(yield* Effect.flip(room(99))).toEqual(
        new RateLimited({ message: "Slow down", retryAfter: 30 }),
      );
    }).pipe(Effect.provide(layer));
  });

  it.effect(
    "maps 5xx and undecodable errors to ServerError and dropped requests to NetworkError",
    () => {
      const replies: Reply[] = [
        { status: 502, body: "<html>Bad gateway</html>" },
        json(418, { error: { kind: "Teapot", message: "?" } }),
        "network",
      ];

      const { layer } = harness((_request, index) => replies[index] ?? "network");

      return Effect.gen(function* () {
        const first = yield* Effect.flip(room(12));
        const second = yield* Effect.flip(room(12));
        const third = yield* Effect.flip(room(12));

        expect(first).toBeInstanceOf(ServerError);
        expect(first).toMatchObject({ status: 502 });
        expect(second).toBeInstanceOf(ServerError);
        expect(second).toMatchObject({ status: 418 });
        expect(third).toBeInstanceOf(NetworkError);
      }).pipe(Effect.provide(layer));
    },
  );

  it.effect("fails a success body that breaks its schema with ServerError", () => {
    const { lastRoomId: _lastRoomId, ...broken } = meFixture;

    const replies: Reply[] = [
      json(200, broken),
      json(200, roomDetailFixture(12)),
      { status: 200, body: "{" },
    ];

    const { layer } = harness((_request, index) => replies[index] ?? "network");

    return Effect.gen(function* () {
      const missingField = yield* Effect.flip(me());

      expect(missingField).toBeInstanceOf(ServerError);
      expect(missingField).toMatchObject({ status: 200 });
      expect((yield* room(12)).room.id).toBe(12);

      const truncated = yield* Effect.flip(room(12));

      expect(truncated).toBeInstanceOf(ServerError);
      expect(truncated).toMatchObject({ status: 200 });
    }).pipe(Effect.provide(layer));
  });
});

describe("readBoot", () => {
  it.effect("parses the boot JSON the shell inlined", () => {
    const script = document.createElement("script");

    script.type = "application/json";
    script.id = "boot";
    script.textContent = JSON.stringify(bootJson);
    document.body.append(script);

    const { layer, seen } = harness(() => "network");

    return Effect.gen(function* () {
      expect(yield* readBoot()).toEqual(bootJson);
      expect(seen).toEqual([]);
    }).pipe(Effect.provide(layer));
  });

  it.effect("falls back to GET /boot and adopts its CSRF token", () => {
    const { layer, seen } = harness((request) =>
      request.url.pathname === "/api/v1/boot"
        ? json(200, { ...bootJson, csrfToken: "boot-token" })
        : json(200, { roomId: 12, unread: false, firstUnreadMessageId: null, unreadCount: 0 }),
    );

    return Effect.gen(function* () {
      expect(yield* readBoot()).toEqual(bootJson);

      yield* markRead(12);

      expect(seen.map((request) => [request.url.pathname, request.csrf])).toEqual([
        ["/api/v1/boot", null],
        ["/api/v1/rooms/12/read", "boot-token"],
      ]);
    }).pipe(Effect.provide(layer));
  });
});

describe("auth response statuses", () => {
  it.effect("keeps credential failures on the sign-in flow instead of navigating on 401", () => {
    const error = { kind: "error", fieldErrors: { base: ["Too many requests or unauthorized."] } };
    const { layer, seen, navigations } = harness(() => json(401, error));

    return Effect.gen(function* () {
      const client = yield* ApiClient;
      yield* client.setCsrfToken("signed-out-token");
      expect(
        yield* passwordSignIn({ emailAddress: "unknown@example.com", password: "wrong" }),
      ).toEqual(error);
      expect(seen[0]?.csrf).toBe("signed-out-token");
      expect(navigations).toEqual([]);
    }).pipe(Effect.provide(layer));
  });

  it.effect(
    "refreshes signed-out CSRF and retries once without treating a bad code as stale",
    () => {
      const error = { kind: "error", fieldErrors: { code: ["That code didn't work."] } };

      const { layer, seen, navigations } = harness((request, index) => {
        if (request.url.pathname === "/api/v1/session/boot") {
          return json(200, { csrfToken: "fresh-signed-out-token" });
        }

        return index === 0 ? { status: 422, body: "" } : json(422, error);
      });

      return Effect.gen(function* () {
        const client = yield* ApiClient;
        yield* client.setCsrfToken("stale-token");
        expect(yield* submitChallenge({ code: "invalid", rememberDevice: false })).toEqual(error);
        expect(seen.map((request) => request.url.pathname)).toEqual([
          "/api/v1/two_factor/challenge",
          "/api/v1/session/boot",
          "/api/v1/two_factor/challenge",
        ]);
        expect(seen[2]?.csrf).toBe("fresh-signed-out-token");
        expect(navigations).toEqual([]);
      }).pipe(Effect.provide(layer));
    },
  );
});

describe("auth token bootstrap", () => {
  it.effect("gets a public CSRF token before the first signed-out write", () => {
    const { layer, seen } = harness((request) =>
      request.method === "GET"
        ? json(200, { csrfToken: "public-token" })
        : json(200, {
            kind: "secondFactorRequired",
            challenge: { methods: ["totp", "recoveryCode"], rememberDevice: true },
          }),
    );

    return Effect.gen(function* () {
      expect(
        (yield* passwordSignIn({ emailAddress: "ada@example.com", password: "secret" })).kind,
      ).toBe("secondFactorRequired");
      expect(seen.map((request) => request.url.pathname)).toEqual([
        "/api/v1/session/boot",
        "/api/v1/session",
      ]);
      expect(seen[1]?.csrf).toBe("public-token");
    }).pipe(Effect.provide(layer));
  });
});
