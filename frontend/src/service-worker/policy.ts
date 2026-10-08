export type RequestPolicy = "precache" | "classic-asset" | "navigation" | "network";

/** Propshaft's classic asset names end in a hexadecimal content fingerprint. */
export function isClassicAsset(path: string): boolean {
  return /^\/assets\/.+-[a-f0-9]{8,}\.[^/]+$/.test(path);
}

export function isSpaAsset(path: string, assetPrefix: string): boolean {
  return (
    path.startsWith(assetPrefix) && /^[^/]+-[\w-]{8,}\.[^/]+$/.test(path.slice(assetPrefix.length))
  );
}

export function requestPolicy(
  request: Pick<Request, "method" | "url" | "mode">,
  origin: string,
  precached: ReadonlySet<string>,
): RequestPolicy {
  const url = new URL(request.url);

  if (request.method !== "GET" || url.origin !== origin) {
    return "network";
  }

  // A navigation always asks the server, even when it names offline.html itself.
  if (request.mode === "navigate") {
    return "navigation";
  }

  if (precached.has(url.href)) {
    return "precache";
  }

  return isClassicAsset(url.pathname) ? "classic-asset" : "network";
}

/** Cache API limits and HTTP cache prohibitions apply to installation and runtime caching. */
export function canCache(response: Pick<Response, "ok" | "status" | "headers">): boolean {
  return (
    response.ok &&
    response.status !== 206 &&
    !response.headers
      .get("Vary")
      ?.split(",")
      .some((field) => field.trim() === "*") &&
    !/(?:^|,)\s*no-store\s*(?:=|,|$)/i.test(response.headers.get("Cache-Control") ?? "")
  );
}
