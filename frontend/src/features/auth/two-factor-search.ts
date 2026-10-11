/**
 * The two-step setup screen's URL query, shared by the router, the shell's enrollment gate and
 * the screen (which loads in its own chunk).
 */

/** The setup screen's query as the URL has it. */
interface RawTwoFactorSetupSearch {
  readonly return_to?: unknown;
  readonly required?: unknown;
}

/**
 * The setup screen's query: the SPA page to go back to once it's set up, and whether the session
 * must set it up before anything else (the enrollment gate, or just after turning it off).
 */
export interface TwoFactorSetupSearch {
  readonly return_to?: string;
  readonly required?: true;
}

export function parseTwoFactorSetupSearch(search: RawTwoFactorSetupSearch): TwoFactorSetupSearch {
  const raw = search.return_to;
  const returnTo = raw === undefined || raw === null ? null : safeReturnPath(String(raw));
  const required = search.required === true || search.required === "true";

  if (returnTo === null) {
    return required ? { required: true } : {};
  }

  return required ? { return_to: returnTo, required: true } : { return_to: returnTo };
}

/** A page of this app to return to (`/app…`, never another origin), else `null`. */
export function safeReturnPath(path: string): string | null {
  if (!/^\/app(?:[/?#]|$)/.test(path) || path.includes("//") || path.includes("\\")) {
    return null;
  }

  return path;
}
