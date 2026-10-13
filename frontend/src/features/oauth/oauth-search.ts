/** The OAuth error page's query, read in the entry (the route validates it before its chunk). */

/** The error page's query: the server's error code, and a client's description if it sent one. */
export interface OAuthErrorSearch {
  error?: string;
  error_description?: string;
}

/** The query as the URL has it. */
interface RawOAuthErrorSearch {
  readonly error?: unknown;
  readonly error_description?: unknown;
}

/** Text trimmed and cut to `max` characters; blank is `undefined`. */
function clipped(text: string, max: number): string | undefined {
  const trimmed = text.trim();

  return trimmed === "" ? undefined : trimmed.slice(0, max);
}

/** `?error=&error_description=`; anything else is dropped. */
export function parseOAuthErrorSearch(search: RawOAuthErrorSearch): OAuthErrorSearch {
  const parsed: OAuthErrorSearch = {};
  const error = clipped(String(search.error ?? ""), 64);
  const description = clipped(String(search.error_description ?? ""), 300);

  if (error !== undefined) parsed.error = error;

  if (description !== undefined) parsed.error_description = description;

  return parsed;
}
