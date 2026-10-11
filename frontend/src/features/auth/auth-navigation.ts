/**
 * Where a sign-in operation's answer leads. Signing in, Google and anything outside the signed-out
 * pages are full page loads (the signed-in app boots afresh); the retained sign-in and challenge
 * URLs the server names stay on their SPA counterparts, so a visitor who started here stays here.
 */

/** A signed-out page's route, under the router's `/app/` base. */
export type SignedOutPage = "/session/new" | "/two_factor/challenge";

/** The SPA page for each retained auth page the server can send a visitor to. */
const SPA_AUTH_PAGES = new Map<string, SignedOutPage>([
  ["/session/new", "/session/new"],
  ["/app/session/new", "/session/new"],
  ["/two_factor_challenge", "/two_factor/challenge"],
  ["/app/two_factor/challenge", "/two_factor/challenge"],
]);

/** The SPA page for a server-named location, or `null` when it is a full page load. */
export function signedOutPageFor(location: string): SignedOutPage | null {
  const url = new URL(location, window.location.href);

  if (url.origin !== window.location.origin) {
    return null;
  }

  return SPA_AUTH_PAGES.get(url.pathname) ?? null;
}

/** The one way these pages leave the SPA; tests replace `assign`. */
export const pageExit = {
  assign(url: string): void {
    window.location.assign(url);
  },
  replace(url: string): void {
    window.location.replace(url);
  },
};

/** The retained first-run page, which the sign-in page hands a fresh install to. */
export const FIRST_RUN_PATH = "/first_run";
