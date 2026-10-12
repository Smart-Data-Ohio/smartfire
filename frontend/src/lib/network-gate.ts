import { store } from "../store/store.ts";

/** Why nothing goes out once another account signed in under the page. */
export const ACCOUNT_CHANGED =
  "You signed in as a different account in another tab. Reload to continue.";

/**
 * The one check right before this page sends anything: every fetch the API client makes (the
 * first try, a CSRF token read or refresh, each retry), the keepalive leave, uploads. Once the
 * server answered the page as another account (`accountChanged`), nothing more goes out, since
 * whatever came back would be theirs. Returns the refusal, a network failure as `fetch` gives one,
 * or `null` when sending is fine.
 */
export function sendRefusal(): TypeError | null {
  return store.getState().accountChanged ? new TypeError(ACCOUNT_CHANGED) : null;
}

/** `fetch` behind `sendRefusal`: refused as a failed network request once nothing may go out. */
export const gatedFetch: typeof globalThis.fetch = (input, init) => {
  const refusal = sendRefusal();

  return refusal === null ? globalThis.fetch(input, init) : Promise.reject(refusal);
};
