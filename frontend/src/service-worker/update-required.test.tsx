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
