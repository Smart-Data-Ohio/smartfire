export const REQUEST_PAGE_BUILD = "smartfire:request-page-build";

export const PAGE_BUILD = "smartfire:page-build";

/** Messages cross a browser boundary; accept only the app's own entry fingerprint. */
export function pageBuildMessage(
  event: Pick<MessageEvent<unknown>, "data">,
  origin: string,
): string | null {
  const value = event.data;

  try {
    if (
      !(value instanceof Object) ||
      !("kind" in value) ||
      value.kind !== PAGE_BUILD ||
      !("page" in value) ||
      String(value.page) !== value.page
    ) {
      return null;
    }

    const url = new URL(value.page);

    return url.origin === origin && /^\/app\/assets\/[^/]+-[\w-]{8,}\.js$/.test(url.pathname)
      ? url.href
      : null;
  } catch {
    return null;
  }
}

export function requestsPageBuild(event: Pick<MessageEvent<unknown>, "data">): boolean {
  const value = event.data;

  return value instanceof Object && "kind" in value && value.kind === REQUEST_PAGE_BUILD;
}
