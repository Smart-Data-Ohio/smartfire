/**
 * The fresh confirmation some writes need (`require_sudo_mode`), in place of the classic page that
 * asks for it. A write the server holds answers `SudoRequired`; the client waits here while the
 * confirmation dialog asks, then replays the write once. Writes that arrive while it asks share
 * the one dialog. Closing it fails them all with `ConfirmationCancelled`, and leaving the screen
 * that sent a write fails that write the same way.
 *
 * Confirming with Google leaves the page. Each Google start names a fresh attempt, which the server
 * keeps with the Google sign-in it starts, in this session. The writes that carry no credential are
 * kept in session storage (method, path and body; never a password, code or token) as one hand-off
 * under that attempt, bound to the person who chose Google and a time limit. The SPA's
 * `/app/sudo/continue` replays them only when the server says it confirmed exactly that attempt,
 * for that same person, in time; the server reads that answer again for the attempt until the page
 * acknowledges it, so a lost answer or an interrupted replay can ask again. Any other answer drops
 * them, and the page says the change wasn't sent. The rest fail with their `SudoRequired` before
 * the page goes, so their forms can keep what isn't secret.
 */
import { Effect, Layer, Option, Schema } from "effect";
import { ApiClient, type ApiRequest } from "../api/client.ts";
import { ConfirmationCancelled, type SudoRequired } from "../api/errors.ts";
import { type ConfirmationTicket, Reauthentication } from "../api/reauthentication.ts";
import { acknowledgeSudo, resumeSudo, startSudoGoogle, submitSudo } from "../api/sudo-endpoints.ts";
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

/** What the page keeps across one Google round trip. */
const Handoff = Schema.Struct({
  /**
   * The attempt Google was started with: the writes go only when the server says it confirmed
   * this attempt, which it does only in the session that started it.
   */
  id: Schema.String,
  /** Who chose Google: nobody else's continuation replays these writes. */
  user: Schema.Number,
  /** When (ms since the epoch) the writes stop being worth sending. */
  expiresAt: Schema.Number,
  /** The SPA page the writes came from. */
  returnTo: Schema.String,
  /** The writes still to send, in order; each leaves as its replay begins. */
  writes: Schema.Array(KeptWrite),
  /** Writes left behind because they carried a credential: their forms ask again. */
  dropped: Schema.Int,
});

type Handoff = typeof Handoff.Type;

/** The hand-off waiting for this person, and how many writes one of theirs held as it ran out. */
interface HandoffClaim {
  readonly handoff: Handoff | null;
  readonly expired: number;
}

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
  /** A fresh Google attempt id. */
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
    // Its screen was left while it was on its way: nobody is there to want it, even when a
    // confirmation landed since.
    if (ticket.screen !== screen()) {
      return leftScreen();
    }

    // Confirmed while this write was on its way: it goes again without asking.
    if (confirmations > ticket.confirmations) {
      return Effect.void;
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

    /** A fresh id for a Google attempt. */
    newAttempt: (): string => newId(),

    /**
     * The page is leaving to confirm Google `attempt`, from `returnTo`. Keeps the writes that carry
     * no credential as a hand-off under the attempt (they stay waiting until the page goes); the
     * others fail with their `SudoRequired` now. Without a person to bind the hand-off to, or
     * storage to keep it in, nothing is kept and every write fails so.
     */
    leave(returnTo: string, attempt: string): void {
      const keep = waiting.filter((entry) => entry.request.secret !== true);
      const user = viewer();

      const stored =
        user !== null &&
        writeHandoff({
          id: attempt,
          user,
          expiresAt: now() + HANDOFF_TTL_MS,
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
     * else, or has run out of time, or doesn't decode is forgotten; `expired` counts the writes
     * this person's ran-out hand-off still held, unsent.
     */
    handoff(): HandoffClaim {
      const handoff = readHandoff();

      if (handoff !== null && handoff.user === viewer() && now() < handoff.expiresAt) {
        return { handoff, expired: 0 };
      }

      writeHandoff(null);

      return {
        handoff: null,
        expired: handoff !== null && handoff.user === viewer() ? handoff.writes.length : 0,
      };
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
  const attempt = gate.newAttempt();
  const reply = yield* startSudoGoogle(attempt);

  if (reply.kind === "navigate") {
    gate.leave(returnTo, attempt);

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
   * Kept writes dropped unsent: the server didn't confirm the attempt they were kept for (another
   * tab's confirmation, a new session, a refusal) or they ran out of time. Their change is lost.
   */
  readonly abandoned: number;
  /**
   * Why the server couldn't say whether the confirmation is fresh (it was unreachable, say). The
   * kept writes are all still kept: try again.
   */
  readonly unreachable: string | null;
}

const anyJson = (json: Schema.Json) => Effect.succeed(json);

/**
 * The SPA's `/app/sudo/continue`: asks the server about the hand-off's Google attempt and, if it
 * confirmed exactly that attempt, replays the writes kept for it, each once and in order, then
 * acknowledges the attempt. Each write leaves storage as its replay begins, so a reload never
 * sends one twice and the rest go after it (the server answers for the attempt again until it is
 * acknowledged); a status read that fails keeps them all for another try. Any other answer drops
 * them, counted in `abandoned`.
 */
export const resumeAfterGoogle = Effect.fn("confirmation.resumeAfterGoogle")(function* (
  gate: ConfirmationGate,
) {
  const { handoff, expired } = gate.handoff();
  const returnTo = spaReturnPath(handoff?.returnTo ?? null);
  const dropped = handoff?.dropped ?? 0;
  const kept = handoff?.writes.length ?? 0;
  const outcome = { returnTo, replayed: 0, failure: null, dropped, abandoned: expired };

  const status = yield* resumeSudo(handoff?.id ?? null).pipe(
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

    return {
      ...outcome,
      confirmed: false,
      abandoned: expired + kept,
      unreachable: null,
    } satisfies GoogleReturn;
  }

  gate.noteConfirmed();

  // Only the server's word that it confirmed this very attempt sends what was kept for it.
  if (handoff === null || reply.attempt !== handoff.id) {
    gate.forget();

    return {
      ...outcome,
      confirmed: true,
      abandoned: expired + kept,
      unreachable: null,
      returnTo:
        handoff === null && reply.retry !== null ? spaReturnPath(reply.retry.returnTo) : returnTo,
    } satisfies GoogleReturn;
  }

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

  // Done with the attempt: the server stops answering for it. If this doesn't arrive, it expires.
  yield* acknowledgeSudo(handoff.id).pipe(Effect.ignore);

  return {
    ...outcome,
    confirmed: true,
    replayed,
    failure,
    unreachable: null,
  } satisfies GoogleReturn;
});
