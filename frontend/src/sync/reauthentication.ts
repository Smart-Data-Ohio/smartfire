/**
 * The fresh confirmation some writes need (`require_sudo_mode`), in place of the classic page that
 * asks for it. A write the server holds answers `SudoRequired`; the client waits here while the
 * confirmation dialog asks, then replays the write once. Writes that arrive while it asks share
 * the one dialog. Closing it fails them all with `ConfirmationCancelled`.
 *
 * Confirming with Google leaves the page. The writes that carry no credential are kept in session
 * storage (method, path and body; never a password, code or token) and replayed by the SPA's
 * `/app/sudo/continue` route once the server says the confirmation is fresh. The rest fail with
 * their `SudoRequired` before the page goes, so their forms can keep what isn't secret.
 */
import { Effect, Layer, Option, Schema } from "effect";
import { ApiClient, type ApiRequest } from "../api/client.ts";
import { ConfirmationCancelled, type SudoRequired } from "../api/errors.ts";
import { Reauthentication } from "../api/reauthentication.ts";
import { resumeSudo, startSudoGoogle, submitSudo } from "../api/sudo-endpoints.ts";
import type { SudoMethod } from "../gen/SudoMethod.ts";
import type { SudoSubmission } from "../gen/SudoSubmission.ts";

/** What the confirmation dialog shows while writes wait on it. */
export interface ConfirmationPrompt {
  /** The ways this account can confirm, in the server's order. */
  readonly methods: readonly SudoMethod[];
  /** How many writes wait on it. */
  readonly writes: number;
  /** How many of them carry a credential, so a Google confirmation can't bring them back. */
  readonly secrets: number;
}

/** A write kept for after the Google round trip: the request exactly as it was sent. */
const KeptWrite = Schema.Struct({
  method: Schema.Literals(["GET", "POST", "PATCH", "PUT", "DELETE"]),
  path: Schema.String,
  root: Schema.optionalKey(Schema.Literal(true)),
  query: Schema.optionalKey(Schema.Record(Schema.String, Schema.String)),
  body: Schema.optionalKey(Schema.Json),
});

/** What the page keeps across the Google round trip. */
const KeptConfirmation = Schema.Struct({
  /** The SPA page the writes came from, if the server's retry doesn't name one. */
  returnTo: Schema.String,
  writes: Schema.Array(KeptWrite),
  /** Writes left behind because they carried a credential: their forms ask again. */
  dropped: Schema.Int,
});

type KeptConfirmation = typeof KeptConfirmation.Type;

const decodeKept = Schema.decodeUnknownOption(Schema.fromJsonString(KeptConfirmation));

/** The session storage key of the writes kept across the Google round trip. */
export const KEPT_KEY = "smartfire:confirmation-pending";

/** Where a confirmed write goes back to when nothing better is known. */
const HOME = "/app/";

/** `path` when it is an SPA page on this origin; the SPA's home otherwise. */
export function spaReturnPath(path: string | null): string {
  if (
    path === null ||
    !(path === "/app" || path.startsWith("/app/")) ||
    path.includes("\\") ||
    path.startsWith("/app/sudo/continue")
  ) {
    return HOME;
  }

  return path;
}

interface Waiting {
  readonly request: ApiRequest;
  readonly required: SudoRequired;
  readonly settle: (outcome: Effect.Effect<void, SudoRequired | ConfirmationCancelled>) => void;
}

/** Session storage, or `null` where the browser refuses it. */
function sessionStore(): Storage | null {
  try {
    return globalThis.sessionStorage ?? null;
  } catch {
    return null;
  }
}

/**
 * The shared confirmation: the `Reauthentication` service the client waits on, and the plain
 * functions the dialog drives. `storage` is where Google round trips keep their writes.
 */
export function makeConfirmationGate(storage: () => Storage | null = sessionStore) {
  let waiting: readonly Waiting[] = [];
  let prompt: ConfirmationPrompt | null = null;
  let confirmations = 0;
  const listeners = new Set<() => void>();

  const publish = (next: readonly Waiting[]) => {
    waiting = next;

    const last = next.at(-1);

    prompt =
      last === undefined
        ? null
        : {
            methods: last.required.reauthentication.methods,
            writes: next.length,
            secrets: next.filter((entry) => entry.request.secret === true).length,
          };

    for (const listener of listeners) {
      listener();
    }
  };

  /** Settles every waiting write with `outcome` (a function, for one per write). */
  const settleAll = (
    outcome: (entry: Waiting) => Effect.Effect<void, SudoRequired | ConfirmationCancelled> | null,
  ) => {
    const settled = waiting;

    publish([]);

    for (const entry of settled) {
      const result = outcome(entry);

      if (result !== null) {
        entry.settle(result);
      }
    }
  };

  const confirm = (required: SudoRequired, request: ApiRequest, sentAfter: number) =>
    // Confirmed while this write was on its way: it goes again without asking.
    confirmations > sentAfter
      ? Effect.void
      : Effect.callback<void, SudoRequired | ConfirmationCancelled>((resume) => {
          const entry: Waiting = { request, required, settle: resume };

          publish([...waiting, entry]);

          // The caller gave up (left the page it was on): it stops waiting, alone.
          return Effect.sync(() => {
            if (waiting.includes(entry)) {
              publish(waiting.filter((other) => other !== entry));
            }
          });
        });

  const layer = Layer.succeed(Reauthentication, {
    confirmations: Effect.sync(() => confirmations),
    confirm,
  });

  return {
    layer,

    subscribe(listener: () => void): () => void {
      listeners.add(listener);

      return () => {
        listeners.delete(listener);
      };
    },

    /** What the dialog shows; `null` while nothing waits. */
    snapshot: (): ConfirmationPrompt | null => prompt,

    /** The person confirmed: every waiting write goes again. */
    confirmed(): void {
      confirmations += 1;
      settleAll(() => Effect.void);
    },

    /** The person closed the dialog: every waiting write fails, unsent. */
    cancel(): void {
      settleAll(() =>
        Effect.fail(new ConfirmationCancelled({ message: "The confirmation was cancelled." })),
      );
    },

    /**
     * The page is leaving to confirm with Google, from `returnTo`. Keeps the writes that carry no
     * credential for the return (they stay waiting until the page goes); the others fail with
     * their `SudoRequired` now.
     */
    leave(returnTo: string): void {
      const keep = waiting.filter((entry) => entry.request.secret !== true);

      const kept: KeptConfirmation = {
        returnTo: spaReturnPath(returnTo),
        // The request as it went, less the flag that keeps credentials out of here.
        writes: keep.map(({ request: { secret, ...write } }) => write),
        dropped: waiting.length - keep.length,
      };

      let stored = false;

      try {
        const store = storage();

        store?.setItem(KEPT_KEY, JSON.stringify(kept));
        stored = store !== null;
      } catch {
        stored = false;
      }

      settleAll((entry) =>
        stored && entry.request.secret !== true ? null : Effect.fail(entry.required),
      );
    },

    /** Takes (and forgets) what the last Google round trip kept; `null` when there's none. */
    takeKept(): KeptConfirmation | null {
      try {
        const store = storage();
        const raw = store?.getItem(KEPT_KEY) ?? null;

        store?.removeItem(KEPT_KEY);

        return raw === null ? null : Option.getOrNull(decodeKept(raw));
      } catch {
        return null;
      }
    },

    /** Counts a confirmation made elsewhere (the Google return), for writes already on their way. */
    noteConfirmed(): void {
      confirmations += 1;
    },
  };
}

export type ConfirmationGate = ReturnType<typeof makeConfirmationGate>;

/** What a password or code submission came to: `null` when confirmed, else the reason to show. */
export const submit = Effect.fn("confirmation.submit")(function* (
  gate: ConfirmationGate,
  submission: SudoSubmission,
) {
  const reply = yield* submitSudo(submission);

  switch (reply.kind) {
    case "confirmed":
      gate.confirmed();

      return null;
    case "error":
      return reply.message;
    case "navigate":
      // The session ended; the client is already on its way to sign in.
      return null;
    case "ready":
      return "Confirmation failed. Try again.";
  }
});

/** Where to go to confirm with Google, or the reason it can't start. */
export type GoogleStart =
  | { readonly kind: "navigate"; readonly location: string }
  | { readonly kind: "error"; readonly message: string };

/**
 * Starts a Google confirmation from the SPA page `returnTo`. On `navigate` the waiting writes are
 * already kept (or failed); the caller leaves for `location`.
 */
export const startGoogle = Effect.fn("confirmation.startGoogle")(function* (
  gate: ConfirmationGate,
  returnTo: string,
) {
  const reply = yield* startSudoGoogle();

  if (reply.kind === "navigate") {
    gate.leave(returnTo);

    return { kind: "navigate", location: reply.location } satisfies GoogleStart;
  }

  return {
    kind: "error",
    message: reply.kind === "error" ? reply.message : "Google confirmation didn't start.",
  } satisfies GoogleStart;
});

/** What the Google return came to, for the continue page to say and where it goes next. */
export interface GoogleReturn {
  /** Whether the server holds a fresh confirmation. */
  readonly confirmed: boolean;
  /** The SPA page to go back to. */
  readonly returnTo: string;
  /** Writes that went again and landed. */
  readonly replayed: number;
  /** The first replayed write's failure, if one failed. */
  readonly failure: string | null;
  /** Writes left behind because they carried a credential. */
  readonly dropped: number;
}

const anyJson = (json: Schema.Json) => Effect.succeed(json);

/**
 * The SPA's `/app/sudo/continue`: asks whether the Google confirmation is fresh and, if it is,
 * replays the writes kept for it, each once and in order. What was kept is forgotten first, so a
 * reload never sends them twice.
 */
export const resumeAfterGoogle = Effect.fn("confirmation.resumeAfterGoogle")(function* (
  gate: ConfirmationGate,
) {
  const kept = gate.takeKept();
  const reply = yield* resumeSudo();
  const fallback = spaReturnPath(kept?.returnTo ?? null);

  if (reply.kind !== "confirmed") {
    return {
      confirmed: false,
      returnTo: fallback,
      replayed: 0,
      failure: null,
      dropped: kept?.dropped ?? 0,
    } satisfies GoogleReturn;
  }

  gate.noteConfirmed();

  const client = yield* ApiClient;
  let replayed = 0;
  let failure: string | null = null;

  for (const write of kept?.writes ?? []) {
    const landed = yield* client.execute(write, anyJson).pipe(
      Effect.match({
        onSuccess: () => null,
        onFailure: (error) => error.message,
      }),
    );

    if (landed === null) {
      replayed += 1;
    } else {
      failure ??= landed;
    }
  }

  return {
    confirmed: true,
    returnTo: reply.retry === null ? fallback : spaReturnPath(reply.retry.returnTo),
    replayed,
    failure,
    dropped: kept?.dropped ?? 0,
  } satisfies GoogleReturn;
});
