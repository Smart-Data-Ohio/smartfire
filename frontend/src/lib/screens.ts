import rows from "../gen/screens.json";

/**
 * One row of the screen map (`crates/spa/src/screens.rs`, written to `src/gen/screens.json`):
 * a classic page and where it lives in the SPA. `ported` says whether the SPA has built it; the
 * server sends people who use the new UI from a ported classic page to its SPA URL.
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
 * A room notification's `thread` and `message_id` query, as the SPA's thread and message routes.
 * `null` when neither is a record id, so the plain room URL keeps the query.
 */
function roomNotificationUrl(spa: string, search: string): string | null {
  let thread: string | undefined;
  let message: string | undefined;
  const rest: string[] = [];

  for (const pair of search.replace(/^\?/, "").split("&")) {
    if (pair === "") {
      continue;
    }

    const eq = pair.indexOf("=");
    const name = eq === -1 ? pair : pair.slice(0, eq);
    const value = eq === -1 ? "" : pair.slice(eq + 1);

    if (name === "classic") {
      continue;
    }

    if (name === "thread" && thread === undefined && isId(value)) {
      thread = String(Number(value));

      continue;
    }

    if (name === "message_id" && message === undefined && isId(value)) {
      message = String(Number(value));

      continue;
    }

    rest.push(pair);
  }

  if (thread === undefined && message === undefined) {
    return null;
  }

  const kept = rest.filter((pair) => {
    const name = pair.split("=")[0] ?? "";

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
export function spaUrlFor(path: string, search = ""): string | null {
  for (const screen of SCREENS) {
    if (!screen.ported) {
      continue;
    }

    const captured = capture(screen.classic, path);

    if (captured !== null) {
      const filled = fill(screen.spa, captured);

      if (screen.classic === "/rooms/:id" && screen.spa === "/app/r/:id") {
        const translated = roomNotificationUrl(filled, search);

        if (translated !== null) {
          return translated;
        }
      }

      return `${filled}${keptSearch(search)}`;
    }
  }

  return null;
}

/**
 * The classic URL of an SPA `path` (`/app/r/12` is `/rooms/12`), ported or not, else `null`:
 * where a destination the SPA hasn't built opens, with a full page load.
 */
export function classicUrlFor(path: string, search = ""): string | null {
  for (const screen of SCREENS) {
    const captured = capture(screen.spa, path);

    if (captured !== null) {
      return `${fill(screen.classic, captured)}${keptSearch(search)}`;
    }
  }

  return null;
}

/** A router location's path as the map spells it: the router's `pathname` leaves out `/app/`. */
function spaPath(pathname: string): string {
  return pathname === "/app" || pathname.startsWith("/app/")
    ? pathname
    : `/app/${pathname.replace(/^\//, "")}`;
}

/**
 * The classic page, with `?classic=1`, for a router location (its `pathname` with or without the
 * `/app/` basepath), or the classic home when nothing maps.
 */
export function classicPageFor(pathname: string, search = ""): string {
  return withClassicBypass(classicUrlFor(spaPath(pathname), search) ?? "/");
}

/** `url` with `?classic=1`, which keeps a person who uses the SPA on the classic page. */
export function withClassicBypass(url: string): string {
  const parsed = new URL(url, "http://x");

  parsed.searchParams.set("classic", "1");

  return `${parsed.pathname}${parsed.search}${parsed.hash}`;
}

/**
 * Where an SPA path the router can't open goes: its classic page (with `?classic=1`) when the map
 * names one, else `null` (a 404).
 */
export function unportedClassicPage(pathname: string, search: string): string | null {
  const classic = classicUrlFor(spaPath(pathname), search);

  return classic === null ? null : withClassicBypass(classic);
}

/** Whether `path` is the SPA's (under `/app/`). */
export function isSpaPath(path: string): boolean {
  return path === BASE.replace(/\/$/, "") || path.startsWith(BASE);
}
