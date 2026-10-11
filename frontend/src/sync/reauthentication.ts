/**
 * The fresh confirmation some writes need (`require_sudo_mode`), in place of the classic page that
 * asks for it. A write the server holds answers `SudoRequired`; the client waits here while the
 * confirmation dialog asks, then replays the write once. Writes that arrive while it asks share
 * the one dialog. Closing it fails them all with `ConfirmationCancelled`, and leaving the screen
 * that sent a write fails that write the same way.
 *
 * Confirming with Google leaves the page. The writes that carry no credential are kept in session
 * storage (method, path and body; never a password, code or token) as one hand-off, bound to the
 * person who chose Google, the server's pending request at that moment, and a time limit. The
 * SPA's `/app/sudo/continue` replays them only when the server says the confirmation is fresh and
 * names that same pending request, for that same person, in time. The rest fail with their
 * `SudoRequired` before the page goes, so their forms can keep what isn't secret.
 */
import { Effect, Layer, Option, Schema } from "effect";
import { ApiClient, type ApiRequest } from "../api/client.ts";
import { ConfirmationCancelled, type SudoRequired } from "../api/errors.ts";
import { type ConfirmationTicket, Reauthentication } from "../api/reauthentication.ts";
import { resumeSudo, startSudoGoogle, submitSudo } from "../api/sudo-endpoints.ts";
import type { SudoMethod } from "../gen/SudoMethod.ts";
import type { SudoResponse } from "../gen/SudoResponse.ts";
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

/** What the page keeps across one Google round trip. */
const Handoff = Schema.Struct({
  /** Made when Google is chosen; the continue page claims the hand-off by it, once. */
  id: Schema.String,
  /** Who chose Google: nobody else's continuation replays these writes. */
  user: Schema.Number,
  /** When (ms since the epoch) the writes stop being worth sending. */
  expiresAt: Schema.Number,
  /**
   * The server's pending request when Google was chosen (`SudoRetry`, less its return): the
   * continuation must name it, which it does once, in the session that recorded it.
   */
  retry: Schema.Struct({ method: Schema.String, path: Schema.String }),
  /** The continuation named it: what's left goes again after a reload without asking it twice. */
  accepted: Schema.Boolean,
  /** The SPA page the writes came from. */
  returnTo: Schema.String,
  /** The writes still to send, in order; each leaves as its replay begins. */
  writes: Schema.Array(KeptWrite),
  /** Writes left behind because they carried a credential: their forms ask again. */
  dropped: Schema.Int,
});

type Handoff = typeof Handoff.Type;

const decodeHandoff = Schema.decodeUnknownOption(Schema.fromJsonString(Handoff));

/** The session storage key of the writes kept across the Google round trip. */
export const KEPT_KEY = "smartfire:confirmation-pending";

/**
 * How long a hand-off lasts: a Google round trip, well inside the server's 15-minute confirmation
 * window.
 */
export const HANDOFF_TTL_MS = 10 * 60_000;

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
  /** The screen that sent it. */
  readonly screen: string;
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

/** The SPA page showing now: a write belongs to the screen it was sent from. */
function currentScreen(): string {
  return globalThis.location?.pathname ?? "";
}

/** What the gate reads from the page; each has a browser default. */
export interface GateOptions {
  /** Where Google round trips keep their writes. */
  readonly storage?: () => Storage | null;
  /** The signed-in person's id; `null` before the boot names them. */
  readonly viewer?: () => number | null;
  readonly now?: () => number;
  /** The screen showing now. */
  readonly screen?: () => string;
  /** A fresh hand-off id. */
  readonly newId?: () => string;
}

const cancelled = (message: string) => Effect.fail(new ConfirmationCancelled({ message }));

/**
 * The shared confirmation: the `Reauthentication` service the client waits on, and the plain
 * functions the dialog, the shell and the continue page drive.
 */
export function makeConfirmationGate({
  storage = sessionStore,
  viewer = () => null,
  now = Date.now,
  screen = currentScreen,
  newId = () => globalThis.crypto.randomUUID(),
}: GateOptions = {}) {
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

  /** Settles each write in `settled` with `outcome` (a function, for one per write). */
  const settle = (
    settled: readonly Waiting[],
    outcome: (entry: Waiting) => Effect.Effect<void, SudoRequired | ConfirmationCancelled> | null,
  ) => {
    for (const entry of settled) {
      const result = outcome(entry);

      if (result !== null) {
        entry.settle(result);
      }
    }
  };

  /** Settles every waiting write, and closes the dialog. */
  const settleAll = (
    outcome: (entry: Waiting) => Effect.Effect<void, SudoRequired | ConfirmationCancelled> | null,
  ) => {
    const settled = waiting;

    publish([]);
    settle(settled, outcome);
  };

  const leftScreen = () => cancelled("The page that asked for this was left.");

  const confirm = (required: SudoRequired, request: ApiRequest, ticket: ConfirmationTicket) => {
    // Confirmed while this write was on its way: it goes again without asking.
    if (confirmations > ticket.confirmations) {
      return Effect.void;
    }

    // Its screen was left while it was on its way: nobody is there to confirm it.
    if (ticket.screen !== screen()) {
      return leftScreen();
    }

    return Effect.callback<void, SudoRequired | ConfirmationCancelled>((resume) => {
      const entry: Waiting = { request, required, screen: ticket.screen, settle: resume };

      publish([...waiting, entry]);

      // The caller gave up: it stops waiting, alone.
      return Effect.sync(() => {
        if (waiting.includes(entry)) {
          publish(waiting.filter((other) => other !== entry));
        }
      });
    });
  };

  const layer = Layer.succeed(Reauthentication, {
    ticket: Effect.sync(() => ({ confirmations, screen: screen() })),
    confirm,
  });

  /** The stored hand-off, if it decodes. */
  const readHandoff = (): Handoff | null => {
    try {
      const raw = storage()?.getItem(KEPT_KEY) ?? null;

      return raw === null ? null : Option.getOrNull(decodeHandoff(raw));
    } catch {
      return null;
    }
  };

  /** Stores `handoff` (or forgets it, for `null`); whether storage took it. */
  const writeHandoff = (handoff: Handoff | null): boolean => {
    try {
      const store = storage();

      if (store === null) {
        return false;
      }

      if (handoff === null) {
        store.removeItem(KEPT_KEY);
      } else {
        store.setItem(KEPT_KEY, JSON.stringify(handoff));
      }

      return true;
    } catch {
      return false;
    }
  };

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
      settleAll(() => cancelled("The confirmation was cancelled."));
    },

    /**
     * The SPA moved to another screen: the writes the screens left behind sent fail, unsent. The
     * dialog closes once none wait.
     */
    screenChanged(): void {
      const here = screen();
      const gone = waiting.filter((entry) => entry.screen !== here);

      if (gone.length === 0) {
        return;
      }

      publish(waiting.filter((entry) => entry.screen === here));
      settle(gone, leftScreen);
    },

    /**
     * The page is leaving to confirm with Google, from `returnTo`. Keeps the writes that carry no
     * credential as a hand-off for the return (they stay waiting until the page goes); the others
     * fail with their `SudoRequired` now. Without a person or a pending request to bind the
     * hand-off to, or storage to keep it in, nothing is kept and every write fails so.
     */
    leave(returnTo: string): void {
      const keep = waiting.filter((entry) => entry.request.secret !== true);
      const retry = waiting.at(-1)?.required.reauthentication.retry ?? null;
      const user = viewer();

      const stored =
        retry !== null &&
        user !== null &&
        writeHandoff({
          id: newId(),
          user,
          expiresAt: now() + HANDOFF_TTL_MS,
          retry: { method: retry.method.toUpperCase(), path: retry.path },
          accepted: false,
          returnTo: spaReturnPath(returnTo),
          // The request as it went, less the flag that keeps credentials out of here.
          writes: keep.map(({ request: { secret, ...write } }) => write),
          dropped: waiting.length - keep.length,
        });

      settleAll((entry) =>
        stored && entry.request.secret !== true ? null : Effect.fail(entry.required),
      );
    },

    /**
     * The hand-off waiting for this person, if one is still good. One that belongs to someone
     * else, or has run out of time, or doesn't decode is forgotten.
     */
    handoff(): Handoff | null {
      const handoff = readHandoff();

      if (handoff === null) {
        writeHandoff(null);

        return null;
      }

      if (handoff.user !== viewer() || now() >= handoff.expiresAt) {
        writeHandoff(null);

        return null;
      }

      return handoff;
    },

    /** Marks hand-off `id` as named by the server's continuation. */
    accept(id: string): void {
      const handoff = readHandoff();

      if (handoff?.id === id) {
        writeHandoff({ ...handoff, accepted: true });
      }
    },

    /**
     * Takes hand-off `id`'s next write to send, leaving the rest stored (or forgetting the hand-off
     * with its last). `null` when there's none left, or the hand-off has changed under it.
     */
    takeNext(id: string): typeof KeptWrite.Type | null {
      const handoff = readHandoff();
      const [next, ...rest] = handoff?.id === id ? handoff.writes : [];

      if (handoff === null || next === undefined) {
        return null;
      }

      // Gone from storage before it's sent: a reload mid-replay never sends it twice.
      if (!writeHandoff(rest.length === 0 ? null : { ...handoff, writes: rest })) {
        return null;
      }

      return next;
    },

    /** Forgets any hand-off. */
    forget(): void {
      writeHandoff(null);
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
  /**
   * Why the server couldn't say whether the confirmation is fresh (it was unreachable, say). The
   * kept writes are all still kept: try again.
   */
  readonly unreachable: string | null;
}

const anyJson = (json: Schema.Json) => Effect.succeed(json);

/** Whether the continuation's pending request is the one `handoff` was bound to. */
function namesHandoff(reply: Extract<SudoResponse, { kind: "confirmed" }>, handoff: Handoff) {
  return (
    reply.retry !== null &&
    reply.retry.method.toUpperCase() === handoff.retry.method &&
    reply.retry.path === handoff.retry.path
  );
}

/**
 * The SPA's `/app/sudo/continue`: asks whether the Google confirmation is fresh and, if it is and
 * the server names the hand-off's pending request, replays the writes kept for it, each once and
 * in order. Each write leaves storage as its replay begins, so a reload never sends one twice; a
 * status read that fails keeps them all for another try. Any other answer forgets them.
 */
export const resumeAfterGoogle = Effect.fn("confirmation.resumeAfterGoogle")(function* (
  gate: ConfirmationGate,
) {
  const handoff = gate.handoff();
  const returnTo = spaReturnPath(handoff?.returnTo ?? null);
  const dropped = handoff?.dropped ?? 0;
  const outcome = { returnTo, replayed: 0, failure: null, dropped, unreachable: null };

  const status = yield* resumeSudo().pipe(
    Effect.match({
      onSuccess: (answer) => ({ answer, unreachable: null }),
      onFailure: (error) => ({ answer: null, unreachable: error.message }),
    }),
  );

  if (status.answer === null) {
    return { ...outcome, confirmed: false, unreachable: status.unreachable } satisfies GoogleReturn;
  }

  const reply = status.answer;

  if (reply.kind !== "confirmed") {
    gate.forget();

    return { ...outcome, confirmed: false } satisfies GoogleReturn;
  }

  gate.noteConfirmed();

  if (handoff === null || !(handoff.accepted || namesHandoff(reply, handoff))) {
    gate.forget();

    return {
      ...outcome,
      confirmed: true,
      returnTo:
        handoff === null && reply.retry !== null ? spaReturnPath(reply.retry.returnTo) : returnTo,
    } satisfies GoogleReturn;
  }

  gate.accept(handoff.id);

  const client = yield* ApiClient;
  let replayed = 0;
  let failure: string | null = null;

  for (let write = gate.takeNext(handoff.id); write !== null; write = gate.takeNext(handoff.id)) {
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

  return { ...outcome, confirmed: true, replayed, failure } satisfies GoogleReturn;
});
