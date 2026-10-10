import rows from "../gen/screens.json";

/**
 * One row of the screen map (`crates/spa/src/screens.rs`, written to `src/gen/screens.json`):
 * a classic page and where it lives in the SPA. `ported` says whether the SPA has built it; the
 * server always sends a signed-in person from a ported classic page to its SPA URL.
 */
export interface Screen {
  readonly endpoint: string;
  /** The classic path pattern: literal segments, and `:param` (optionally after a prefix: `@:id`). */
  readonly classic: string;
  /** The SPA path pattern, with the same parameters. */
  readonly spa: string;
  readonly ported: boolean;
}

export const SCREENS: readonly Screen[] = rows;

/** The SPA's base path (`/app/`), which the router's `basepath` is too. */
const BASE = import.meta.env.BASE_URL;

function segments(path: string): string[] {
  return path.split("/").filter((segment) => segment !== "");
}

/** A record id, as the server matches one: a positive integer below 2^53, digits only. */
function isId(segment: string): boolean {
  return /^\d+$/.test(segment) && Number.isSafeInteger(Number(segment)) && Number(segment) > 0;
}

/** `path`'s parameters by name when it matches `pattern`, else `null`. */
function capture(pattern: string, path: string): Map<string, string> | null {
  const expected = segments(pattern);
  const actual = segments(path);

  if (expected.length !== actual.length) {
    return null;
  }

  const captured = new Map<string, string>();

  for (const [index, part] of expected.entries()) {
    const segment = actual[index] ?? "";
    const colon = part.indexOf(":");

    if (colon === -1) {
      if (part !== segment) {
        return null;
      }

      continue;
    }

    const prefix = part.slice(0, colon);
    const value = segment.slice(prefix.length);

    if (!segment.startsWith(prefix) || !isId(value)) {
      return null;
    }

    captured.set(part.slice(colon + 1), String(Number(value)));
  }

  return captured;
}

function fill(pattern: string, captured: Map<string, string>): string {
  const filled = segments(pattern).map((part) => {
    const colon = part.indexOf(":");

    return colon === -1
      ? part
      : `${part.slice(0, colon)}${captured.get(part.slice(colon + 1)) ?? ""}`;
  });

  const path = `/${filled.join("/")}`;

  return pattern.endsWith("/") && path !== "/" ? `${path}/` : path;
}

/**
 * One application/x-www-form-urlencoded component (`+` is a space). `null` when the encoding is
 * broken, so the value cannot be a record id. Matches the server's query decoder.
 */
function queryComponent(raw: string): string | null {
  try {
    return decodeURIComponent(raw.replace(/\+/g, " "));
  } catch {
    return null;
  }
}

/** A record id from a query component, percent-decoded first (`thread=%39` is 9). */
function recordId(raw: string): string | null {
  const decoded = queryComponent(raw);

  return decoded !== null && isId(decoded) ? String(Number(decoded)) : null;
}

/**
 * A room notification's `thread` and `message_id` query, as the SPA's thread and message routes.
 * `null` when neither effective value is a record id, so the plain room URL keeps the query.
 * `thread` is the first value (the classic thread panel's `URLSearchParams.get`). A non-empty
 * `thread` makes `message_id` the first value too. With no thread, `message_id` is the last
 * value (the room controller's params). Only that value is parsed; another duplicate is not a
 * fallback when it isn't an id. The same rules as the server translator.
 */
function roomNotificationUrl(spa: string, search: string): string | null {
  let threadRaw: string | undefined;
  let messageFirst: string | undefined;
  let messageLast: string | undefined;
  const rest: string[] = [];

  for (const pair of search.replace(/^\?/, "").split("&")) {
    if (pair === "") {
      continue;
    }

    const eq = pair.indexOf("=");
    const rawName = eq === -1 ? pair : pair.slice(0, eq);
    const rawValue = eq === -1 ? "" : pair.slice(eq + 1);
    const name = queryComponent(rawName);

    if (name === "classic") {
      continue;
    }

    if (name === "thread") {
      if (threadRaw === undefined) {
        threadRaw = rawValue;
      }

      continue;
    }

    if (name === "message_id") {
      if (messageFirst === undefined) {
        messageFirst = rawValue;
      }

      messageLast = rawValue;

      continue;
    }

    rest.push(pair);
  }

  const threadDecoded = threadRaw === undefined ? null : queryComponent(threadRaw);
  const threadOpens = threadDecoded !== null && threadDecoded !== "";
  const messageRaw = threadOpens ? messageFirst : messageLast;
  const thread = threadRaw === undefined ? undefined : (recordId(threadRaw) ?? undefined);
  const message = messageRaw === undefined ? undefined : (recordId(messageRaw) ?? undefined);

  if (thread === undefined && message === undefined) {
    return null;
  }

  const kept = rest.filter((pair) => {
    const rawName = pair.split("=")[0] ?? "";
    const name = queryComponent(rawName) ?? rawName;

    return name !== "thread" && name !== "message_id";
  });

  if (thread !== undefined) {
    if (message !== undefined) {
      kept.unshift(`m=${message}`);
    }

    const query = kept.length === 0 ? "" : `?${kept.join("&")}`;

    return `${spa}/t/${thread}${query}`;
  }

  const query = kept.length === 0 ? "" : `?${kept.join("&")}`;

  return `${spa}/m/${message}${query}`;
}

/**
 * `search` less any `classic` parameter, keeping every other pair as it was written.
 * The server's room redirect does the same when it leaves the query on the plain room URL.
 */
function rawKeptSearch(search: string): string {
  const kept = search
    .replace(/^\?/, "")
    .split("&")
    .filter((pair) => pair !== "" && (pair.split("=")[0] ?? "") !== "classic");

  return kept.length === 0 ? "" : `?${kept.join("&")}`;
}

/**
 * A thread's content fragment's `search` as the SPA's thread route reads it: `message_id` (the
 * last value) becomes `m`, dropped when it isn't a record id. Other pairs carry over as written,
 * less `classic`. The same rules as the server's `thread_anchor_url`.
 */
function threadAnchorSearch(search: string): string {
  let anchor: string | null = null;
  const rest: string[] = [];

  for (const pair of search.replace(/^\?/, "").split("&")) {
    if (pair === "") {
      continue;
    }

    const eq = pair.indexOf("=");
    const rawName = eq === -1 ? pair : pair.slice(0, eq);
    const rawValue = eq === -1 ? "" : pair.slice(eq + 1);
    const name = queryComponent(rawName);

    if (name === "classic") {
      continue;
    }

    if (name === "message_id") {
      anchor = recordId(rawValue);

      continue;
    }

    rest.push(pair);
  }

  const kept = anchor === null ? rest : [`m=${anchor}`, ...rest];

  return kept.length === 0 ? "" : `?${kept.join("&")}`;
}

/** `search` (with or without its `?`) less any `classic` parameter, as `?...` or "". */
function keptSearch(search: string): string {
  const params = new URLSearchParams(search);

  params.delete("classic");

  const kept = params.toString();

  return kept === "" ? "" : `?${kept}`;
}

/**
 * The SPA URL of a classic `path` the SPA has ported (`/rooms/12` is `/app/r/12`), else `null`.
 * The search carries over, less `classic`.
 */
export function spaUrlFor(path: string, search = "", viewerId?: number): string | null {
  for (const screen of SCREENS) {
    if (!screen.ported) {
      continue;
    }

    const captured = capture(screen.classic, path);

    if (captured !== null) {
      const filled = fill(screen.spa, captured);

      if (filled === `/app/people/${viewerId}`) {
        return `/app/settings${keptSearch(search)}`;
      }

      if (screen.classic === "/rooms/:id" && screen.spa === "/app/r/:id") {
        const translated = roomNotificationUrl(filled, search);

        if (translated !== null) {
          return translated;
        }

        return `${filled}${rawKeptSearch(search)}`;
      }

      if (screen.endpoint === "channel_threads#content") {
        return `${filled}${threadAnchorSearch(search)}`;
      }

      return `${filled}${keptSearch(search)}`;
    }
  }

  return null;
}

/** Whether `path` is the SPA's (under `/app/`). */
export function isSpaPath(path: string): boolean {
  return path === BASE.replace(/\/$/, "") || path.startsWith(BASE);
}
