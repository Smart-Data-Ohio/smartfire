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
 * acknowledges it, so a lost answer or an interrupted replay can ask again. Any other answer keeps
 * them unsent and asks the person to confirm again (a new attempt; a concurrent cookie write can
 * erase a finished one); they go only when the person discards them, another person or session
 * signs in, or they run out of time. A duplicated tab carries a copy of the hand-off, so the tabs
 * of this browser replay an attempt one at a time (a Web Lock) and mark each write in local
 * storage as it goes: a write marked by one tab is never sent by another. Where the lock can't be
 * held, nothing replays on its own: the person's "Send again" in one tab is the claim. The rest
 * fail with their `SudoRequired` before the page goes, so their forms can keep what isn't secret.
 */
import { Effect, Layer, Option, Schema } from "effect";
import { ApiClient, type ApiRequest } from "../api/client.ts";
import { ConfirmationCancelled, SudoRequired } from "../api/errors.ts";
import { type ConfirmationTicket, Reauthentication } from "../api/reauthentication.ts";
import type { SudoResponse, SudoRetry as SudoRetrySchema } from "../api/schema/sudo.ts";
import { acknowledgeSudo, resumeSudo, startSudoGoogle, submitSudo } from "../api/sudo-endpoints.ts";
import type { SudoMethod } from "../gen/SudoMethod.ts";
import type { SudoSubmission } from "../gen/SudoSubmission.ts";

/** What the confirmation dialog shows while writes wait on it. */
export interface ConfirmationPrompt {
  /** The ways this account can confirm, in the server's order. */
  readonly methods: readonly SudoMethod[];
  /** How many writes wait on it. */
  readonly writes: number;
  /**
   * How many of them a Google confirmation can't bring back: they carry a credential, or a
   * multipart body.
   */
  readonly secrets: number;
  /**
   * How many of them are full-page actions (connecting Google, say) checked before they leave:
   * after a Google confirmation the person does them again.
   */
  readonly again: number;
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
  /** The batch: its writes' marks across tabs and its lock are by this id. */
  id: Schema.String,
  /**
   * The latest attempt Google was started with: the writes go only when the server says it
   * confirmed this attempt, which it does only in the session that started it.
   */
  attempt: Schema.String,
  /** The signed-in session that started it: in any other, the writes are dropped. */
  session: Schema.NullOr(Schema.Number),
  /** Who chose Google: nobody else's continuation replays these writes. */
  user: Schema.Number,
  /** When (ms since the epoch) the writes stop being worth sending. */
  expiresAt: Schema.Number,
  /** The first kept write's place in the batch: each write's mark across tabs. */
  next: Schema.Int,
  /** The SPA page the writes came from. */
  returnTo: Schema.String,
  /** The writes still to send, in order; each leaves as its replay begins. */
  writes: Schema.Array(KeptWrite),
  /** Writes left behind because they carried a credential: their forms ask again. */
  dropped: Schema.Int,
  /** Full-page actions that were waiting too: the person does them again. */
  again: Schema.optionalKey(Schema.Int),
});

type Handoff = typeof Handoff.Type;

/** A kept write on its way, with its place in the batch. */
interface KeptTurn {
  readonly write: typeof KeptWrite.Type;
  readonly index: number;
}

/** The hand-off waiting for this person, and how many writes one of theirs held as it ran out. */
interface HandoffClaim {
  readonly handoff: Handoff | null;
  readonly expired: number;
}

const decodeHandoff = Schema.decodeUnknownOption(Schema.fromJsonString(Handoff));

/** The session storage key of the writes kept across the Google round trip. */
export const KEPT_KEY = "smartfire:confirmation-pending";

/** The local storage key prefix of the writes an attempt's replays have sent, in any tab. */
export const SENT_PREFIX = "smartfire:confirmation-sent:";

/** Which of an attempt's writes (by place in the batch) a tab has sent, and until when it matters. */
/**
 * Batch marks every tab reads: `sent` are the writes a tab claimed (none replays them again),
 * `delivered` those whose send the server answered.
 */
const SentRecord = Schema.Struct({
  expiresAt: Schema.Number,
  sent: Schema.Array(Schema.Int),
  delivered: Schema.optionalKey(Schema.Array(Schema.Int)),
});

const decodeSent = Schema.decodeUnknownOption(Schema.fromJsonString(SentRecord));

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
    path.startsWith("/app/sudo/continue") ||
    path.startsWith(SUDO_PAGE)
  ) {
    return HOME;
  }

  return path;
}

interface Waiting {
  /** The held write; `null` for a full-page action checked before it leaves (`confirmFirst`). */
  readonly request: ApiRequest | null;
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

/** Local storage, or `null` where the browser refuses it. */
function localStore(): Storage | null {
  try {
    return globalThis.localStorage ?? null;
  } catch {
    return null;
  }
}

/**
 * Holds the browser's Web Lock `name` until the returned release runs; `null` where the browser
 * has no Web Locks or won't grant this one, or `signal` aborts the wait for it, so nothing
 * replays.
 */
function webLock(name: string, signal?: AbortSignal): Promise<(() => void) | null> {
  if (!("locks" in globalThis.navigator)) {
    return Promise.resolve(null);
  }

  return new Promise((acquired) => {
    globalThis.navigator.locks
      .request(
        name,
        signal === undefined ? {} : { signal },
        () => new Promise<void>((release) => acquired(() => release())),
      )
      .catch(() => acquired(null));
  });
}

/** The SPA page showing now: a write belongs to the screen it was sent from. */
function currentScreen(): string {
  return globalThis.location?.pathname ?? "";
}

/** What the gate reads from the page; each has a browser default. */
export interface GateOptions {
  /** Where Google round trips keep their writes. */
  readonly storage?: () => Storage | null;
  /** Where every tab marks the writes it replayed. */
  readonly shared?: () => Storage | null;
  /**
   * Takes a lock every tab of this browser shares; resolves with its release, or `null` (also
   * once `signal` aborts the wait).
   */
  readonly lock?: (name: string, signal?: AbortSignal) => Promise<(() => void) | null>;
  /** The signed-in person's id; `null` before the boot names them. */
  readonly viewer?: () => number | null;
  readonly now?: () => number;
  /** The screen showing now. */
  readonly screen?: () => string;
  /** A fresh Google attempt id. */
  readonly newId?: () => string;
}

/**
 * Whether a held write can wait in storage through a Google round trip: not one carrying a
 * credential, nor a multipart one (its `FormData` doesn't survive storage). Those fail instead,
 * for their forms to send again.
 */
const keepable = (request: ApiRequest) => request.secret !== true && request.form === undefined;

/** A held write that can't wait in storage (not a full-page action, which isn't a write). */
const unkeepable = (entry: Waiting) => entry.request !== null && !keepable(entry.request);

const cancelled = (message: string) => Effect.fail(new ConfirmationCancelled({ message }));

/**
 * The shared confirmation: the `Reauthentication` service the client waits on, and the plain
 * functions the dialog, the shell and the continue page drive.
 */
export function makeConfirmationGate({
  storage = sessionStore,
  shared = localStore,
  lock = webLock,
  viewer = () => null,
  now = Date.now,
  screen = currentScreen,
  newId = () => globalThis.crypto.randomUUID(),
}: GateOptions = {}) {
  let waiting: readonly Waiting[] = [];
  let prompt: ConfirmationPrompt | null = null;
  let confirmations = 0;
  // Writes a return that couldn't finish reported lost, for the one that does to say.
  let lost = 0;
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
            secrets: next.filter(unkeepable).length,
            again: next.filter((entry) => entry.request === null).length,
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

    return wait(request, required, ticket.screen);
  };

  /** Waits on the dialog with `request` (or a full-page action, `null`) from `from`. */
  const wait = (request: ApiRequest | null, required: SudoRequired, from: string) =>
    Effect.callback<void, SudoRequired | ConfirmationCancelled>((resume) => {
      const entry: Waiting = { request, required, screen: from, settle: resume };

      publish([...waiting, entry]);

      // The caller gave up: it stops waiting, alone.
      return Effect.sync(() => {
        if (waiting.includes(entry)) {
          publish(waiting.filter((other) => other !== entry));
        }
      });
    });

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

  /** The writes attempt `id`'s replays have sent, in any tab, by place in the batch. */
  const sentRecord = (id: string) => {
    const raw = shared()?.getItem(`${SENT_PREFIX}${id}`) ?? null;

    return raw === null ? null : Option.getOrNull(decodeSent(raw));
  };

  /** Drops the marks of attempts whose writes are no longer worth sending anywhere. */
  const forgetSentRecords = () => {
    try {
      const store = shared();

      if (store === null) {
        return;
      }

      const keys = Array.from({ length: store.length }, (_, index) => store.key(index));

      for (const key of keys) {
        if (key?.startsWith(SENT_PREFIX) === true) {
          const record = Option.getOrNull(decodeSent(store.getItem(key) ?? ""));

          if (record === null || now() >= record.expiresAt) {
            store.removeItem(key);
          }
        }
      }
    } catch {
      // Nothing to tidy where storage refuses.
    }
  };

  return {
    layer,

    /**
     * Holds this browser's lock on batch `id`'s replay; resolves with its release, or `null`
     * where it can't be held or `signal` aborts the wait.
     */
    exclusive: (id: string, signal?: AbortSignal): Promise<(() => void) | null> =>
      lock(`smartfire:confirmation:${id}`, signal),

    /** How many writes this person's stored hand-off keeps, and their page; reads only. */
    kept(): { readonly writes: number; readonly returnTo: string } | null {
      const handoff = readHandoff();

      return handoff === null || handoff.user !== viewer()
        ? null
        : { writes: handoff.writes.length, returnTo: spaReturnPath(handoff.returnTo) };
    },

    /**
     * Marks write `index` of batch `id` as claimed, for every tab: `"claimed"` when the mark is
     * stored (read back) and this tab may send it, `"sent"` when a tab already sent it and the
     * server answered, `"unconfirmed"` when a tab claimed it but its answer never came (it isn't
     * sent again), `"unavailable"` when storage can't say, or the batch is past `expiresAt` (its
     * marks are forgotten then), so it isn't sent.
     */
    claimSent(
      id: string,
      index: number,
      expiresAt: number,
    ): "claimed" | "sent" | "unconfirmed" | "unavailable" {
      try {
        const store = shared();

        if (store === null || now() >= expiresAt) {
          return "unavailable";
        }

        const record = sentRecord(id);

        if (record?.sent.includes(index) === true) {
          return record.delivered?.includes(index) === true ? "sent" : "unconfirmed";
        }

        store.setItem(
          `${SENT_PREFIX}${id}`,
          JSON.stringify({ ...record, expiresAt, sent: [...(record?.sent ?? []), index] }),
        );

        return sentRecord(id)?.sent.includes(index) === true ? "claimed" : "unavailable";
      } catch {
        return "unavailable";
      }
    },

    /** Whether writes can be marked as sent for every tab. */
    canMark(): boolean {
      try {
        const store = shared();

        if (store === null) {
          return false;
        }

        store.setItem(`${SENT_PREFIX}probe`, "");
        store.removeItem(`${SENT_PREFIX}probe`);

        return true;
      } catch {
        return false;
      }
    },

    /** The server took the send of write `index` of batch `id`: marks it delivered. */
    markDelivered(id: string, index: number): void {
      try {
        const record = sentRecord(id);

        if (record !== null) {
          shared()?.setItem(
            `${SENT_PREFIX}${id}`,
            JSON.stringify({ ...record, delivered: [...(record.delivered ?? []), index] }),
          );
        }
      } catch {
        // Unmarked, it reads as a send whose answer never came: true enough.
      }
    },

    /**
     * Of `handoff`'s kept writes, how many a tab already claimed (`claimed`, none goes again),
     * and how many of those never had the server's answer (`unconfirmed`).
     */
    claims(handoff: Handoff) {
      try {
        const record = sentRecord(handoff.id);
        const indices = handoff.writes.map((_, offset) => handoff.next + offset);
        const claimed = indices.filter((index) => record?.sent.includes(index) === true);

        return {
          claimed: claimed.length,
          unconfirmed: claimed.filter((index) => record?.delivered?.includes(index) !== true)
            .length,
        };
      } catch {
        return { claimed: 0, unconfirmed: 0 };
      }
    },

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
     * A full-page action needs a fresh confirmation the server says isn't there
     * (`reauthentication`): waits on the dialog with the held writes, from the screen showing
     * now. Succeeds once confirmed here; fails if cancelled, the screen is left, or the person
     * leaves for Google (they do it again once back).
     */
    confirmNavigation: (reauthentication: SudoRequired["reauthentication"]) =>
      wait(
        null,
        new SudoRequired({ message: "Confirm your password to continue", reauthentication }),
        screen(),
      ),

    /** A fresh id for a Google attempt. */
    newAttempt: (): string => newId(),

    /**
     * The page is leaving to confirm Google `attempt`, from `returnTo`, in signed-in `session`.
     * Keeps the writes that carry no credential as a hand-off under the attempt (they stay
     * waiting until the page goes); the
     * others fail with their `SudoRequired` now. Without a person to bind the hand-off to, or
     * storage to keep it in, nothing is kept and every write fails so.
     */
    leave(returnTo: string, attempt: string, session: number | null): void {
      const keep = waiting.flatMap((entry) =>
        entry.request !== null && keepable(entry.request) ? [entry.request] : [],
      );

      const user = viewer();

      const stored =
        user !== null &&
        writeHandoff({
          id: newId(),
          attempt,
          session,
          user,
          expiresAt: now() + HANDOFF_TTL_MS,
          next: 0,
          returnTo: spaReturnPath(returnTo),
          // The request as it went, less the flag that keeps credentials out of here.
          writes: keep.map(({ secret, form, ...write }) => write),
          dropped: waiting.filter(unkeepable).length,
          again: waiting.filter((entry) => entry.request === null).length,
        });

      // Full-page actions don't wait in storage: they fail now, and the person does them again.
      settleAll((entry) =>
        stored && entry.request !== null && keepable(entry.request)
          ? null
          : Effect.fail(entry.required),
      );
    },

    /**
     * The hand-off waiting for this person, if one is still good. One that belongs to someone
     * else, or has run out of time, or doesn't decode is forgotten; `expired` counts the writes
     * this person's ran-out hand-off still held, unsent.
     */
    handoff(): HandoffClaim {
      const handoff = readHandoff();

      forgetSentRecords();

      if (handoff !== null && handoff.user === viewer() && now() < handoff.expiresAt) {
        return { handoff, expired: 0 };
      }

      writeHandoff(null);

      return {
        handoff: null,
        expired: handoff !== null && handoff.user === viewer() ? handoff.writes.length : 0,
      };
    },

    /** Hand-off `id`'s next write to send, with its place in the batch; `null` when none. */
    nextKept(id: string): KeptTurn | null {
      const handoff = readHandoff();
      const next = handoff?.id === id ? handoff.writes[0] : undefined;

      return handoff === null || next === undefined ? null : { write: next, index: handoff.next };
    },

    /**
     * Takes write `index` of hand-off `id` out of storage, leaving the rest (or forgetting the
     * hand-off with its last). Whether it was the next and left: gone before it's sent, a reload
     * mid-replay never sends it twice.
     */
    take(id: string, index: number): boolean {
      const handoff = readHandoff();

      if (handoff?.id !== id || handoff.next !== index) {
        return false;
      }

      const rest = handoff.writes.slice(1);

      return writeHandoff(
        rest.length === 0 ? null : { ...handoff, next: handoff.next + 1, writes: rest },
      );
    },

    /**
     * The kept writes are going with Google `attempt` now: the hand-off moves to it, keeping its
     * time limit (the marks of what other tabs sent last only that long). Whether there was one,
     * still in time, to move.
     */
    rekey(attempt: string): boolean {
      const handoff = readHandoff();

      return handoff !== null && now() < handoff.expiresAt && writeHandoff({ ...handoff, attempt });
    },

    /** Forgets any hand-off. */
    forget(): void {
      writeHandoff(null);
    },

    /**
     * Carries the writes `result` reports lost to the return the continue page announces: one
     * that couldn't finish (the server unreachable, or writes held) passes them on; the next
     * that does reports them all, once.
     */
    carryLost(result: GoogleReturn): GoogleReturn {
      if (result.unreachable !== null || result.held !== null) {
        lost += result.abandoned;

        return result;
      }

      const abandoned = result.abandoned + lost;

      lost = 0;

      return { ...result, abandoned };
    },

    /** The person discarded what was kept: nothing lost is left to report. */
    dropLost(): void {
      lost = 0;
    },

    /** Counts a confirmation made elsewhere (the Google return), for writes already on their way. */
    noteConfirmed(): void {
      confirmations += 1;
    },
  };
}

export type ConfirmationGate = ReturnType<typeof makeConfirmationGate>;

/**
 * Before a full-page action the server confirms first (connecting Google, say): whether a fresh
 * confirmation is in place, asking for one in the dialog when it isn't. `"go"` once there is one;
 * `"stay"` when the person cancelled, or left to confirm with Google (they do it again once back);
 * `"unavailable"` with the reason when the server couldn't be asked, or signed out (`null`: the
 * client is already on its way to the sign-in page).
 */
export const confirmFirst = Effect.fn("confirmation.confirmFirst")(function* (
  gate: ConfirmationGate,
): Effect.fn.Return<ConfirmFirst, never, ApiClient> {
  const reply = yield* resumeSudo(null).pipe(
    Effect.match({
      onSuccess: (answer) => answer,
      onFailure: (error) => ({ kind: "error", message: error.message }) as const,
    }),
  );

  switch (reply.kind) {
    case "confirmed":
      return { kind: "go" } as const;
    case "ready":
      return yield* gate.confirmNavigation(reply.reauthentication).pipe(
        Effect.match({
          onSuccess: () => ({ kind: "go" }) as const,
          onFailure: () => ({ kind: "stay" }) as const,
        }),
      );
    case "navigate":
      return { kind: "unavailable", reason: null } as const;
    case "error":
      return { kind: "unavailable", reason: reply.message } as const;
  }
});

type SudoRetry = typeof SudoRetrySchema.Type;

/** What `confirmFirst` came to. */
export type ConfirmFirst =
  | { readonly kind: "go" }
  | { readonly kind: "stay" }
  | { readonly kind: "unavailable"; readonly reason: string | null };

/** What `/app/sudo/new` reads first: the server's answer, or why it couldn't be had. */
export type SudoStatus =
  | typeof SudoResponse.Type
  | { readonly kind: "unreachable"; readonly message: string };

/**
 * `/app/sudo/new`'s read: whether the confirmation is fresh (`confirmed`, with what the server
 * held for after it) or what it can be confirmed with (`ready`). A fresh one is consumed: what
 * the server held is the page's to continue with now.
 */
export const sudoStatus = Effect.fn("confirmation.status")(function* () {
  return yield* resumeSudo(null).pipe(
    Effect.catch((error) =>
      Effect.succeed({ kind: "unreachable", message: error.message } satisfies SudoStatus),
    ),
  );
});

/** What a credential sent from `/app/sudo/new` came to. */
export type SudoHere =
  | { readonly kind: "confirmed"; readonly retry: SudoRetry | null }
  | { readonly kind: "refused"; readonly message: string }
  | { readonly kind: "signedOut" };

/**
 * Confirms from `/app/sudo/new`, where no write waits in memory: the server's held request comes
 * back with the confirmation (`retry`) for the page to continue with. A dialog waiting meanwhile
 * (another screen's write) goes on too.
 */
export const confirmHere = Effect.fn("confirmation.confirmHere")(function* (
  gate: ConfirmationGate,
  submission: SudoSubmission,
) {
  const reply = yield* submitSudo(submission);

  switch (reply.kind) {
    case "confirmed":
      gate.confirmed();

      return { kind: "confirmed", retry: reply.retry } satisfies SudoHere;
    case "error":
      return { kind: "refused", message: reply.message } satisfies SudoHere;
    case "navigate":
      // The session ended; the client is already on its way to sign in.
      return { kind: "signedOut" } satisfies SudoHere;
    case "ready":
      return { kind: "refused", message: "Confirmation failed. Try again." } satisfies SudoHere;
  }
});

/**
 * Starts Google from `/app/sudo/new`. No writes wait (and a hand-off another page kept stays
 * as it is): the continue page asks the server whether it confirmed, and what it held.
 */
export const googleHere = Effect.fn("confirmation.googleHere")(function* () {
  const reply = yield* startSudoGoogle(null);

  if (reply.kind === "navigate") {
    return { kind: "navigate", location: reply.location } satisfies GoogleStart;
  }

  return {
    kind: "error",
    message: reply.kind === "error" ? reply.message : "Google confirmation didn't start.",
  } satisfies GoogleStart;
});

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
    gate.leave(returnTo, attempt, reply.session ?? null);

    return { kind: "navigate", location: reply.location } satisfies GoogleStart;
  }

  return {
    kind: "error",
    message: reply.kind === "error" ? reply.message : "Google confirmation didn't start.",
  } satisfies GoogleStart;
});

/**
 * Confirms with Google again for the writes the continue page still keeps, under a new attempt.
 * On `navigate` the caller leaves for `location`; the continue page replays them on its return.
 */
export const confirmAgain = Effect.fn("confirmation.confirmAgain")(function* (
  gate: ConfirmationGate,
) {
  if (gate.handoff().handoff === null) {
    return { kind: "error", message: EXPIRED_COPY } satisfies GoogleStart;
  }

  const attempt = gate.newAttempt();
  const reply = yield* startSudoGoogle(attempt);

  if (reply.kind === "navigate") {
    return gate.rekey(attempt)
      ? ({ kind: "navigate", location: reply.location } satisfies GoogleStart)
      : ({ kind: "error", message: EXPIRED_COPY } satisfies GoogleStart);
  }

  return {
    kind: "error",
    message: reply.kind === "error" ? reply.message : "Google confirmation didn't start.",
  } satisfies GoogleStart;
});

const EXPIRED_COPY = "This change waited too long to be confirmed. Discard it and make it again.";

/** The person gives up on the writes the continue page keeps. */
export function discardKept(gate: ConfirmationGate): void {
  gate.forget();
  gate.dropLost();
}

/** What the Google return came to, for the continue page to say and where it goes next. */
export interface GoogleReturn {
  /** Whether the server holds a fresh confirmation. */
  readonly confirmed: boolean;
  /** The SPA page to go back to. */
  readonly returnTo: string;
  /**
   * A page to load in full instead (no hand-off: the server's held request was a page the person
   * asked for, `GET`, before it asked them to confirm); `null` to go back to `returnTo`.
   */
  readonly follow: string | null;
  /** Writes that went again and landed. */
  readonly replayed: number;
  /** The first replayed write's failure, if one failed. */
  readonly failure: string | null;
  /** Writes left behind because they carried a credential. */
  readonly dropped: number;
  /** Full-page actions (connecting Google, say) that waited too: the person does them again. */
  readonly again: number;
  /**
   * Kept writes dropped unsent: the server didn't confirm the attempt they were kept for (another
   * tab's confirmation, a new session, a refusal) or they ran out of time. Their change is lost.
   */
  readonly abandoned: number;
  /** Kept writes another tab of this browser already sent (a duplicated tab's copy). */
  readonly elsewhere: number;
  /**
   * Kept writes a tab (another, or this one before a reload) started sending but never heard
   * back about: they aren't sent again, and may or may not have saved.
   */
  readonly unconfirmed: number;
  /**
   * Why `kept` writes wait, unsent: `"confirm"` when the server didn't confirm their attempt (ask
   * the person to confirm again), `"claim"` when it did but this tab couldn't hold the browser's
   * lock on them and mark them sent for every tab (nothing else keeps another tab from sending
   * them too: try again, or discard them); `null` when none wait.
   */
  readonly held: "confirm" | "claim" | null;
  /** How many writes wait while `held`. */
  readonly kept: number;
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
 * acknowledged); a status read that fails keeps them all for another try. Tabs of this browser
 * replay a batch one at a time, and a write goes only once its mark (read back) says this tab
 * sent it: one another tab sent is skipped (`elsewhere`). Where the lock or the mark can't be
 * had, nothing goes and the writes wait (`"claim"`) for another try. Any other answer keeps them
 * for the person to confirm again, unless the session changed (`abandoned`). Once `signal` aborts
 * (the person discarded them, or left), it stops waiting for the lock and sends nothing more;
 * `onWait` hears when another tab holds the lock a while.
 */
export const resumeAfterGoogle = Effect.fn("confirmation.resumeAfterGoogle")(function* (
  gate: ConfirmationGate,
  options: { readonly signal?: AbortSignal; readonly onWait?: () => void } = {},
) {
  return gate.carryLost(yield* resumeOnce(gate, options));
});

const resumeOnce = Effect.fnUntraced(function* (
  gate: ConfirmationGate,
  { signal, onWait }: { readonly signal?: AbortSignal; readonly onWait?: () => void },
) {
  const { handoff, expired } = gate.handoff();

  if (signal?.aborted === true) {
    return untouched(handoff?.returnTo ?? null, expired);
  }

  if (handoff === null) {
    return yield* resume(gate, null, expired, false, signal);
  }

  return yield* Effect.acquireUseRelease(
    Effect.promise(() => {
      const claim = gate.exclusive(handoff.id, signal);
      const notice = setTimeout(() => onWait?.(), WAIT_NOTICE_MS);

      return claim.finally(() => clearTimeout(notice));
    }),
    (release) => {
      if (signal?.aborted === true) {
        return Effect.succeed(untouched(handoff.returnTo, expired));
      }

      // Re-read under the lock: this tab's hand-off as it is now, and what ran out meanwhile.
      const now = gate.handoff();

      return resume(
        gate,
        now.handoff,
        expired + now.expired,
        release !== null && gate.canMark(),
        signal,
        handoff.returnTo,
      );
    },
    (release) => Effect.sync(() => release?.()),
  );
});

/**
 * Where a confirmation with nothing kept goes, by what the server held (`retry`): a page asked for
 * (`GET`) loads again; a change (never sent again from here) goes back to the page it came from
 * to make again. Unconfirmed with something held, back to `/app/sudo/new` to confirm again.
 */
export function afterConfirming(
  reply: typeof SudoResponse.Type,
  fallback: string,
): Pick<GoogleReturn, "returnTo" | "follow"> {
  if (reply.kind === "confirmed" && reply.retry !== null) {
    const { method, path, returnTo } = reply.retry;

    return {
      returnTo: spaReturnPath(returnTo),
      follow: method.toUpperCase() === "GET" && pageLoad(path) ? path : null,
    };
  }

  if (reply.kind === "ready" && reply.reauthentication.retry !== null) {
    return { returnTo: SUDO_PAGE, follow: null };
  }

  return { returnTo: fallback, follow: null };
}

/** The SPA page that asks for a confirmation in full (where the server's sudo gate sends pages). */
export const SUDO_PAGE = "/app/sudo/new";

/** Whether `path` is a page of this site to load (not the API, not another origin). */
const pageLoad = (path: string) =>
  path.startsWith("/") && !path.startsWith("//") && !path.startsWith("/api/");

/** How long the lock may take before the page says another tab holds it. */
const WAIT_NOTICE_MS = 300;

/** What a return the person gave up on (or left) came to: nothing asked, nothing sent. */
const untouched = (returnTo: string | null, expired: number): GoogleReturn => ({
  confirmed: false,
  returnTo: spaReturnPath(returnTo),
  follow: null,
  replayed: 0,
  failure: null,
  dropped: 0,
  again: 0,
  abandoned: expired,
  elsewhere: 0,
  unconfirmed: 0,
  held: null,
  kept: 0,
  unreachable: null,
});

const resume = Effect.fnUntraced(function* (
  gate: ConfirmationGate,
  handoff: Handoff | null,
  expired: number,
  /** Whether this tab holds the browser's lock on the batch and can mark writes sent. */
  exclusive: boolean,
  /** Aborts once the person discards the writes, or leaves: nothing more goes. */
  signal?: AbortSignal,
  /** The page the hand-off came from, when it ran out before this read. */
  from: string | null = null,
) {
  const returnTo = spaReturnPath(handoff?.returnTo ?? from);
  const dropped = handoff?.dropped ?? 0;

  const outcome = {
    returnTo,
    follow: null,
    replayed: 0,
    failure: null,
    dropped,
    again: handoff?.again ?? 0,
    abandoned: expired,
    elsewhere: 0,
    unconfirmed: 0,
    held: null,
    kept: 0,
    unreachable: null,
  } satisfies Omit<GoogleReturn, "confirmed">;

  const status = yield* resumeSudo(handoff?.attempt ?? null).pipe(
    Effect.match({
      onSuccess: (answer) => ({ answer, unreachable: null }),
      onFailure: (error) => ({ answer: null, unreachable: error.message }),
    }),
  );

  if (status.answer === null) {
    return { ...outcome, confirmed: false, unreachable: status.unreachable } satisfies GoogleReturn;
  }

  const reply = status.answer;
  const confirmed = reply.kind === "confirmed";

  if (confirmed) {
    gate.noteConfirmed();
  }

  // Nothing kept here: the server may hold what the person asked for before it asked them to
  // confirm (`/app/sudo/new`).
  if (handoff === null) {
    return { ...outcome, ...afterConfirming(reply, returnTo), confirmed } satisfies GoogleReturn;
  }

  // Writes another tab already claimed aren't lost, whatever this answer says.
  const { claimed, unconfirmed } = gate.claims(handoff);
  const elsewhere = claimed - unconfirmed;
  const unsent = handoff.writes.length - claimed;
  const session = reply.kind === "confirmed" || reply.kind === "ready" ? reply.session : undefined;

  // Signed in again since Google was chosen: what was kept belongs to the session that ended.
  if ((session ?? null) !== handoff.session) {
    gate.forget();

    return {
      ...outcome,
      confirmed,
      abandoned: expired + unsent,
      elsewhere,
      unconfirmed,
    } satisfies GoogleReturn;
  }

  // Only the server's word that it confirmed this very attempt sends what was kept for it.
  // Anything else (a refusal, another tab's attempt, a record a concurrent cookie write erased)
  // keeps the writes for the person to confirm again.
  if (reply.kind !== "confirmed" || reply.attempt !== handoff.attempt) {
    if (unsent === 0) {
      gate.forget();
    }

    return {
      ...outcome,
      confirmed,
      elsewhere,
      unconfirmed,
      held: unsent === 0 ? null : "confirm",
      kept: unsent,
    } satisfies GoogleReturn;
  }

  // Confirmed, but nothing keeps another tab from sending the same writes: none go from here.
  if (!exclusive) {
    return {
      ...outcome,
      confirmed,
      elsewhere,
      unconfirmed,
      held: "claim",
      kept: unsent,
    } satisfies GoogleReturn;
  }

  const client = yield* ApiClient;
  let replayed = 0;
  let sentElsewhere = 0;
  let unanswered = 0;
  let failure: string | null = null;

  // Discard, or leaving the page: whatever the replay would send next (a token refresh's retry,
  // say) doesn't go. A request already on its way can't be called back.
  const stopped = Effect.callback<"stopped">((resume) => {
    if (signal === undefined) return;

    if (signal.aborted) {
      resume(Effect.succeed("stopped"));

      return;
    }

    signal.addEventListener("abort", () => resume(Effect.succeed("stopped")), { once: true });
  });

  for (const [offset, write] of handoff.writes.entries()) {
    const index = handoff.next + offset;
    const rest = { ...handoff, next: index, writes: handoff.writes.slice(offset) };

    // Discarded, or left: the rest stay as they are, unsent.
    if (signal?.aborted === true) {
      return {
        ...outcome,
        confirmed,
        replayed,
        elsewhere: sentElsewhere,
        unconfirmed: unanswered,
        failure,
      };
    }

    const claim = gate.claimSent(handoff.id, index, handoff.expiresAt);

    // Without its mark another tab could send it too: it and the rest wait, unsent (or, run out,
    // are lost).
    if (claim === "unavailable") {
      const unsentRest = rest.writes.length - gate.claims(rest).claimed;
      const ranOut = gate.handoff().handoff === null;

      return {
        ...outcome,
        confirmed,
        replayed,
        abandoned: expired + (ranOut ? unsentRest : 0),
        elsewhere: sentElsewhere,
        unconfirmed: unanswered,
        failure,
        held: ranOut || unsentRest === 0 ? null : "claim",
        kept: ranOut ? 0 : unsentRest,
      } satisfies GoogleReturn;
    }

    // This tab holds the lock and the mark: the write is its to send. Taking it out of storage
    // only tidies up (a reload finds it marked either way), so it goes even if that fails.
    gate.take(handoff.id, index);

    if (claim === "sent") {
      sentElsewhere += 1;
    } else if (claim === "unconfirmed") {
      unanswered += 1;
    } else {
      const landed = yield* client.execute(write, anyJson).pipe(
        Effect.match({
          onSuccess: () => null,
          onFailure: (error) => error.message,
        }),
        Effect.race(stopped),
      );

      if (landed === "stopped") {
        continue;
      }

      // Only a send the server took is marked delivered; a refused one reads as unconfirmed.
      if (landed === null) {
        gate.markDelivered(handoff.id, index);
        replayed += 1;
      } else {
        failure ??= landed;
      }
    }
  }

  // A copy storage wouldn't let go of is all marked sent by now; one that kept no writes (only
  // full-page actions waited) is done with too.
  if (handoff.writes.length === 0 || gate.nextKept(handoff.id) !== null) {
    gate.forget();
  }

  // Done with the attempt: the server stops answering for it. If this doesn't arrive, it expires.
  yield* acknowledgeSudo(handoff.attempt).pipe(Effect.ignore);

  return {
    ...outcome,
    confirmed,
    replayed,
    elsewhere: sentElsewhere,
    unconfirmed: unanswered,
    failure,
  } satisfies GoogleReturn;
});
