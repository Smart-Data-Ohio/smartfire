import { act, render, renderHook, screen } from "@testing-library/react";
import { Component, type ReactNode, Suspense } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

beforeEach(() => vi.resetModules());

class ExistingErrorHandler extends Component<
  { readonly children: ReactNode; readonly onError: (error: Error) => void },
  { readonly failed: boolean }
> {
  override state = { failed: false };

  static getDerivedStateFromError() {
    return { failed: true };
  }

  override componentDidCatch(error: Error): void {
    this.props.onError(error);
  }

  override render() {
    return this.state.failed ? <p>Existing error handling</p> : this.props.children;
  }
}

describe("module-load update recovery", () => {
  it("catches an actual React.lazy rejection without replacing the rest of the page", async () => {
    const update = await import("./update-required.ts");
    const { lazyForUpdate } = await import("./lazy.ts");

    const Missing = lazyForUpdate<Record<string, never>>(() =>
      Promise.reject(new TypeError("Failed to fetch dynamically imported module")),
    );

    const state = renderHook(update.useAppUpdateRequired);

    expect(state.result.current).toBe(false);
    await act(async () => {
      render(
        <>
          <p>Current page and draft</p>
          <Suspense fallback={<span>Loading feature</span>}>
            <Missing />
          </Suspense>
        </>,
      );
    });

    expect(screen.getByText("Current page and draft").isConnected).toBe(true);
    expect(screen.queryByText("Loading feature")).toBeNull();
    expect(state.result.current).toBe(true);
  });

  it("signals failed non-React loaders while preserving their existing error handling", async () => {
    const update = await import("./update-required.ts");
    const failure = new TypeError("Failed to fetch dynamically imported module");

    await expect(update.loadForUpdate(() => Promise.reject(failure))).rejects.toBe(failure);
    expect(update.appUpdateRequired()).toBe(true);
  });

  it("reads Vite's resource-error payload without discarding its originating rejection", async () => {
    const update = await import("./update-required.ts");
    const target = new EventTarget();
    const dispose = update.watchAppUpdateErrors(target);
    const state = renderHook(update.useAppUpdateRequired);

    const event = Object.assign(new Event("vite:preloadError", { cancelable: true }), {
      payload: new TypeError("Failed to fetch dynamically imported module"),
    });

    act(() => {
      target.dispatchEvent(event);
    });

    expect(event.defaultPrevented).toBe(false);
    expect(state.result.current).toBe(true);
    dispose();
  });

  it.each([
    new TypeError(
      "Failed to fetch dynamically imported module: https://smartfire.test/app/chunk.js",
    ),
    new TypeError("error loading dynamically imported module: https://smartfire.test/app/chunk.js"),
    new TypeError("Importing a module script failed."),
    new Error("Unable to preload CSS for https://smartfire.test/app/chunk.css"),
  ])(
    "recovers recognized Chrome, Firefox, Safari and Vite CSS resource failures: %s",
    async (failure) => {
      const update = await import("./update-required.ts");
      const { lazyForUpdate } = await import("./lazy.ts");
      const Missing = lazyForUpdate<Record<string, never>>(() => Promise.reject(failure));

      await act(async () => {
        render(
          <Suspense fallback={null}>
            <Missing />
          </Suspense>,
        );
      });

      expect(update.appUpdateRequired()).toBe(true);
    },
  );

  it.each([
    new Error("Module initialization failed"),
    new ReferenceError("missingBinding is not defined"),
    new TypeError("Cannot read properties of undefined"),
  ])(
    "passes ordinary module-evaluation failures to existing lazy error handling: %s",
    async (failure) => {
      const update = await import("./update-required.ts");
      const { lazyForUpdate } = await import("./lazy.ts");
      const Broken = lazyForUpdate<Record<string, never>>(() => Promise.reject(failure));
      let caught: Error | null = null;

      await act(async () => {
        render(
          <ExistingErrorHandler
            onError={(error) => {
              caught = error;
            }}
          >
            <Suspense fallback={null}>
              <Broken />
            </Suspense>
          </ExistingErrorHandler>,
          { onCaughtError: () => undefined },
        );
      });

      expect(caught).toBe(failure);
      expect(screen.getByText("Existing error handling").isConnected).toBe(true);
      expect(update.appUpdateRequired()).toBe(false);
      await expect(update.loadForUpdate(() => Promise.reject(failure))).rejects.toBe(failure);
      expect(() => update.ignoreModuleResourceLoadError(failure)).toThrow(failure);

      const target = new EventTarget();
      const dispose = update.watchAppUpdateErrors(target);

      const event = Object.assign(new Event("vite:preloadError", { cancelable: true }), {
        payload: failure,
      });

      target.dispatchEvent(event);
      expect(event.defaultPrevented).toBe(false);
      expect(update.appUpdateRequired()).toBe(false);
      dispose();
    },
  );

  it("ignores unvalidated preload payloads and unrelated API-style fetch failures", async () => {
    const update = await import("./update-required.ts");
    const target = new EventTarget();
    const dispose = update.watchAppUpdateErrors(target);

    for (const payload of [
      undefined,
      "Failed to fetch dynamically imported module",
      { name: "TypeError", message: "Failed to fetch dynamically imported module" },
      new TypeError("Failed to fetch"),
      new Error("Failed to fetch dynamically imported module"),
    ]) {
      const event = Object.assign(new Event("vite:preloadError", { cancelable: true }), {
        payload,
      });

      target.dispatchEvent(event);
      expect(event.defaultPrevented).toBe(false);
      expect(update.appUpdateRequired()).toBe(false);
    }

    dispose();
  });

  it("does not treat an API failure after a successful module import as a load failure", async () => {
    const update = await import("./update-required.ts");
    const apiFailure = new Error("API request failed");

    await expect(
      update
        .loadForUpdate(() => Promise.resolve({ loaded: true }))
        .then(() => {
          throw apiFailure;
        }),
    ).rejects.toBe(apiFailure);
    expect(update.appUpdateRequired()).toBe(false);
  });
});

function pageWithEntry(src: string | null): Document {
  const doc = document.implementation.createHTMLDocument("Smartfire");
  const base = doc.createElement("base");

  base.href = "https://smartfire.test/app/r/1";
  doc.head.append(base);

  if (src !== null) {
    const script = doc.createElement("script");

    script.type = "module";
    script.setAttribute("src", src);
    doc.head.append(script);
  }

  return doc;
}

const servedPage = (src: string) =>
  `<!doctype html><html><head><script type="module" crossorigin src="${src}"></script></head><body></body></html>`;

describe("telling a deploy apart from a failed load", () => {
  it("confirms a newer build only when the server's page names a different entry", async () => {
    const { newerBuildAvailable } = await import("./update-required.ts");
    const current = pageWithEntry("/app/assets/index-AAAA1111.js");
    const requests: Array<[string, RequestInit | undefined]> = [];

    const serve = (src: string) => (url: RequestInfo | URL, init?: RequestInit) => {
      requests.push([String(url), init]);

      return Promise.resolve(new Response(servedPage(src), { status: 200 }));
    };

    await expect(
      newerBuildAvailable(serve("/app/assets/index-BBBB2222.js"), current),
    ).resolves.toBe(true);
    await expect(
      newerBuildAvailable(serve("/app/assets/index-AAAA1111.js"), current),
    ).resolves.toBe(false);
    expect(requests[0]?.[0]).toBe("https://smartfire.test/app/");
    expect(requests[0]?.[1]?.cache).toBe("no-store");
  });

  it("counts offline, error responses and pages without an entry as no newer build", async () => {
    const { newerBuildAvailable } = await import("./update-required.ts");
    const current = pageWithEntry("/app/assets/index-AAAA1111.js");

    await expect(
      newerBuildAvailable(() => Promise.reject(new TypeError("Failed to fetch")), current),
    ).resolves.toBe(false);
    await expect(
      newerBuildAvailable(
        () =>
          Promise.resolve(
            new Response(servedPage("/app/assets/index-BBBB2222.js"), { status: 503 }),
          ),
        current,
      ),
    ).resolves.toBe(false);
    await expect(
      newerBuildAvailable(
        () => Promise.resolve(new Response("<!doctype html><p>Sign in</p>", { status: 200 })),
        current,
      ),
    ).resolves.toBe(false);

    const network = vi.fn<typeof fetch>();

    await expect(newerBuildAvailable(network, pageWithEntry(null))).resolves.toBe(false);
    expect(network).not.toHaveBeenCalled();
  });

  it("shows the plain wording at once, says updated only once confirmed, and re-checks after a failure", async () => {
    const update = await import("./update-required.ts");
    const { UpdateBanner } = await import("../features/shell/update-banner.tsx");
    let answer!: (newer: boolean) => void;

    const pending = () =>
      new Promise<boolean>((resolve) => {
        answer = resolve;
      });

    render(<UpdateBanner />);
    const banner = screen.getByRole("status", { hidden: true });

    expect(banner.dataset.open).toBe("false");

    // The plain wording shows at once, before the probe answers.
    act(() => update.requireAppUpdate(pending));
    expect(update.appUpdateRequired()).toBe(true);
    expect(banner.dataset.open).toBe("true");
    expect(banner.textContent).toContain("Couldn't load part of Smartfire. Reload to try again.");

    await act(async () => answer(false));
    expect(update.appUpdateKind()).toBe("failed");

    act(() => update.requireAppUpdate(pending));
    await act(async () => answer(true));
    expect(update.appUpdateKind()).toBe("updated");
    expect(banner.textContent).toContain(
      "Smartfire has been updated. Reload to get the latest version.",
    );

    const check = vi.fn(() => Promise.resolve(false));

    act(() => update.requireAppUpdate(check));
    expect(check).not.toHaveBeenCalled();
    expect(update.appUpdateKind()).toBe("updated");
  });

  it("bounds a probe whose response never arrives, or whose body never finishes", async () => {
    vi.useFakeTimers();

    try {
      const { newerBuildAvailable } = await import("./update-required.ts");
      const current = pageWithEntry("/app/assets/index-AAAA1111.js");
      const signals: AbortSignal[] = [];

      const hangingHeaders = (_url: RequestInfo | URL, init?: RequestInit) => {
        if (init?.signal) signals.push(init.signal);

        return new Promise<Response>(() => undefined);
      };

      const headers = newerBuildAvailable(hangingHeaders, current, 5000);

      await vi.advanceTimersByTimeAsync(5000);
      await expect(headers).resolves.toBe(false);
      expect(signals[0]?.aborted).toBe(true);

      const hangingBody = (_url: RequestInfo | URL, init?: RequestInit) => {
        if (init?.signal) signals.push(init.signal);

        const body = new ReadableStream<Uint8Array>({ start: () => undefined });

        return Promise.resolve(new Response(body, { status: 200 }));
      };

      const bodyProbe = newerBuildAvailable(hangingBody, current, 5000);

      await vi.advanceTimersByTimeAsync(5000);
      await expect(bodyProbe).resolves.toBe(false);
      expect(signals[1]?.aborted).toBe(true);
    } finally {
      vi.useRealTimers();
    }
  });

  it("keeps the plain strip and re-checks later when a check never settles", async () => {
    vi.useFakeTimers();

    try {
      const update = await import("./update-required.ts");
      const stalled = vi.fn(() => new Promise<boolean>(() => undefined));

      update.requireAppUpdate(stalled);
      expect(update.appUpdateKind()).toBe("failed");

      await vi.advanceTimersByTimeAsync(update.UPDATE_PROBE_TIMEOUT_MS + 1000);

      const confirmed = vi.fn(() => Promise.resolve(true));

      update.requireAppUpdate(confirmed);
      await vi.advanceTimersByTimeAsync(0);
      expect(confirmed).toHaveBeenCalledTimes(1);
      expect(update.appUpdateKind()).toBe("updated");
    } finally {
      vi.useRealTimers();
    }
  });
});
