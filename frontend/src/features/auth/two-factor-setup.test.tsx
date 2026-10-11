import {
  createBrowserHistory,
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { useEffect } from "react";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { SETUP_WRONG, SIGNED_OUT_LOCATION } from "../../../mock/s2/account.ts";
import { addressBar } from "../../api/testing.ts";
import { settings } from "../../sync/settings.ts";
import { type SetupAnswer, twoFactorSetup } from "../../sync/two-factor-setup.ts";
import { installMockNetwork } from "../../test/mock-network.ts";
import { useTwoFactorSetupGate } from "./two-factor-gate.ts";
import { parseTwoFactorSetupSearch, safeReturnPath } from "./two-factor-search.ts";
import { EXPIRED_MESSAGE, TwoFactorSetupPage } from "./two-factor-setup.tsx";

const network = installMockNetwork();

// A desktop: the code field takes focus as the setup appears. (Not `vi.stubGlobal`: unstubbing
// would also take back the mock network's `fetch`.)
beforeAll(() => {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    value: (media: string) =>
      Object.assign(new EventTarget(), {
        media,
        matches: media === "(pointer: fine)",
        onchange: null,
      }),
  });
});

afterAll(() => {
  Reflect.deleteProperty(window, "matchMedia");
  network.restore();
});

/** Sets the mock account's two-step state (`/__mock/two-factor`). */
async function twoFactor(change: {
  readonly enrolled?: boolean;
  readonly required?: boolean;
  readonly expire?: boolean;
}): Promise<void> {
  const response = await network.server.handle({
    method: "POST",
    path: "/__mock/two-factor",
    body: change,
  });

  expect(response.status).toBe(200);
}

/** A page in the shell: it listens for the gate and makes a request, as the app's screens do. */
function GatedRoom() {
  useTwoFactorSetupGate();

  // Two requests refused together, as the shell's are: only one may route.
  useEffect(() => {
    settings.account().catch(() => undefined);
    settings.account().catch(() => undefined);
  }, []);

  return <p>Room destination</p>;
}

/** The window's history at `from`, with `path` pushed on top (by the router's history, so it can
 * step back and forth over both). */
function windowHistory(from: string, path: string) {
  window.history.replaceState(null, "", from);

  const history = createBrowserHistory();

  history.push(path);

  return history;
}

/**
 * Mounts the routes at `path`. With `from`, in the window's own history, `from` a page behind it:
 * only the browser's history lets a blocker hold Back.
 */
async function mount(path: string, from?: string) {
  const root = createRootRoute({ component: Outlet });

  const setup = createRoute({
    getParentRoute: () => root,
    path: "two_factor/setup",
    validateSearch: parseTwoFactorSetupSearch,
    component: TwoFactorSetupPage,
  });

  const room = createRoute({
    getParentRoute: () => root,
    path: "r/$roomId",
    component: GatedRoom,
  });

  const history =
    from === undefined
      ? createMemoryHistory({ initialEntries: [path] })
      : windowHistory(from, path);

  const router = createRouter({
    routeTree: root.addChildren([setup, room]),
    basepath: "/app",
    history,
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return router;
}

let leave = vi.spyOn(twoFactorSetup, "leave");

beforeEach(() => {
  network.server.reset();
  leave = vi.spyOn(twoFactorSetup, "leave").mockImplementation(() => undefined);
});

afterEach(() => vi.restoreAllMocks());

function codeField(): HTMLInputElement {
  return screen.getByRole<HTMLInputElement>("textbox", { name: "Authenticator code" });
}

function manualKey(): string {
  return document.querySelector("[data-manual-key]")?.textContent ?? "";
}

async function enter(code: string): Promise<void> {
  fireEvent.change(codeField(), { target: { value: code } });
  fireEvent.click(screen.getByRole("button", { name: "Verify and continue" }));
}

describe("the setup screen", () => {
  it("shows the setup's shape while the key loads", async () => {
    const pending = Promise.withResolvers<SetupAnswer>();

    vi.spyOn(twoFactorSetup, "read").mockReturnValue(pending.promise);
    await mount("/app/two_factor/setup");

    expect(screen.getByRole("status", { name: "Loading your setup" })).toBeTruthy();
    expect(screen.getByRole("main").getAttribute("aria-busy")).toBe("true");
    expect(screen.getByRole("heading", { name: "Set up two-step sign-in" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Sign out" })).toBeTruthy();
    await act(async () => pending.resolve({ kind: "navigate", location: "/app/" }));
  });

  it("shows the server's QR code as an image, and the manual key to copy", async () => {
    const writeText = vi.fn(() => Promise.resolve());

    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
    await twoFactor({ enrolled: false, required: true });
    await mount("/app/two_factor/setup?required=true");

    const qr = await screen.findByRole<HTMLImageElement>("img", {
      name: "QR code for your authenticator app",
    });

    // The SVG is drawn as an image: nothing of it is inserted into the page as markup.
    expect(qr.src.startsWith("data:image/svg+xml;charset=utf-8,%3Csvg")).toBe(true);
    expect(document.querySelector(".two-factor-card svg:not(.icon)")).toBeNull();

    const key = manualKey();

    expect(key).toMatch(/^[0-9A-F]{4}( [0-9A-F]{4}){3}$/);
    fireEvent.click(screen.getByRole("button", { name: "Copy key" }));
    expect(writeText).toHaveBeenCalledWith(key.replaceAll(" ", ""));
    // Required setup has no way back, only sign out.
    expect(screen.queryByRole("button", { name: "Back to settings" })).toBeNull();
    expect(screen.getByRole("button", { name: "Sign out" })).toBeTruthy();
  });

  it("keeps the key and empties the field for a wrong code", async () => {
    await twoFactor({ enrolled: false, required: true });
    await mount("/app/two_factor/setup");
    await screen.findByRole("img", { name: "QR code for your authenticator app" });

    const key = manualKey();

    await enter("000000");
    expect(await screen.findByText(SETUP_WRONG)).toBeTruthy();
    expect(codeField().value).toBe("");
    expect(codeField().getAttribute("aria-invalid")).toBe("true");
    expect(manualKey()).toBe(key);

    await enter("");
    expect(await screen.findByText("Enter the 6-digit code from your app.")).toBeTruthy();
  });

  it("says when the setup expired and shows its new key", async () => {
    await twoFactor({ enrolled: false, required: true });
    await mount("/app/two_factor/setup");
    await screen.findByRole("img", { name: "QR code for your authenticator app" });

    const key = manualKey();

    await twoFactor({ expire: true });
    await enter("123456");
    expect(await screen.findByText(EXPIRED_MESSAGE)).toBeTruthy();
    expect(manualKey()).not.toBe(key);

    // The new key's code goes through.
    await enter("123 456");
    expect(await screen.findByRole("heading", { name: "Save your backup codes" })).toBeTruthy();
  });

  it("shows the backup codes once, then returns where setup was opened from", async () => {
    await twoFactor({ enrolled: false });
    await mount("/app/two_factor/setup?return_to=%2Fapp%2Fsettings%2Fsecurity");
    await screen.findByRole("img", { name: "QR code for your authenticator app" });
    expect(screen.getByRole("button", { name: "Back to settings" })).toBeTruthy();

    await enter("123456");

    const heading = await screen.findByRole("heading", { name: "Save your backup codes" });

    expect(document.activeElement).toBe(heading);
    expect(screen.getByText("Signed out your other devices")).toBeTruthy();

    const codes = screen.getByRole("list", { name: "Backup codes" });

    expect(codes.querySelectorAll("li")).toHaveLength(10);
    expect(screen.getByRole("button", { name: "Copy all" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Download" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "I've saved these codes" }));
    expect(leave).toHaveBeenCalledWith("/app/settings/security");
    expect(screen.queryByRole("list", { name: "Backup codes" })).toBeNull();
    // Nobody can show them again: setup now sends an enrolled account to its profile.
    expect(await twoFactorSetup.read()).toEqual({
      kind: "navigate",
      location: "/users/me/profile",
    });
  });

  it("sends an already enrolled account to its settings", async () => {
    await mount("/app/two_factor/setup");
    await waitFor(() => expect(leave).toHaveBeenCalledWith("/app/settings"));
  });

  it("signs out while setup is required", async () => {
    await twoFactor({ enrolled: false, required: true });
    await mount("/app/two_factor/setup?required=true");
    await screen.findByRole("img", { name: "QR code for your authenticator app" });

    fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
    await waitFor(() => expect(leave).toHaveBeenCalledWith(SIGNED_OUT_LOCATION));
  });

  it("ignores an answer that arrives after the screen has gone", async () => {
    const pending = Promise.withResolvers<SetupAnswer>();

    vi.spyOn(twoFactorSetup, "read").mockReturnValue(pending.promise);

    const router = await mount("/app/two_factor/setup");

    await act(() => router.navigate({ to: "/r/$roomId", params: { roomId: 4 } }));
    expect(screen.getByText("Room destination")).toBeTruthy();
    await act(async () => pending.resolve({ kind: "navigate", location: "/app/settings" }));
    expect(leave).not.toHaveBeenCalled();
  });
});

describe("leaving the setup screen", () => {
  it("drops a late sign-in answer once the screen has gone, through the real client", async () => {
    const assign = vi.spyOn(addressBar, "assign").mockImplementation(() => undefined);
    const handle = network.server.handle.bind(network.server);
    const release = Promise.withResolvers<void>();

    // The setup's read is slow, then answers that the session has ended.
    vi.spyOn(network.server, "handle").mockImplementation(async (request) => {
      if (request.method === "GET" && request.path.startsWith("/api/v1/two_factor/setup")) {
        await release.promise;

        return { status: 401, json: { kind: "navigate", location: "/app/session/new" } };
      }

      return handle(request);
    });

    const router = await mount("/app/two_factor/setup");

    await act(() => router.navigate({ to: "/r/$roomId", params: { roomId: 4 } }));
    expect(screen.getByText("Room destination")).toBeTruthy();
    await act(async () => {
      release.resolve();
      await new Promise((settled) => setTimeout(settled, 20));
    });
    expect(assign).not.toHaveBeenCalled();
    expect(leave).not.toHaveBeenCalled();
    expect(router.state.location.pathname).toBe("/r/4");
  });

  it("follows the same answer while the screen is up", async () => {
    const assign = vi.spyOn(addressBar, "assign").mockImplementation(() => undefined);
    const handle = network.server.handle.bind(network.server);

    vi.spyOn(network.server, "handle").mockImplementation((request) =>
      request.method === "GET" && request.path.startsWith("/api/v1/two_factor/setup")
        ? Promise.resolve({ status: 401, json: { kind: "navigate", location: "/app/session/new" } })
        : handle(request),
    );
    await mount("/app/two_factor/setup");
    await waitFor(() => expect(leave).toHaveBeenCalledTimes(1));
    expect(assign).not.toHaveBeenCalled();
  });

  it("asks before Back leaves unsaved backup codes", async () => {
    await twoFactor({ enrolled: false });

    const router = await mount("/app/two_factor/setup?return_to=%2Fapp%2Fr%2F4", "/app/r/4");

    await screen.findByRole("img", { name: "QR code for your authenticator app" });
    await enter("123456");
    await screen.findByRole("list", { name: "Backup codes" });

    await act(async () => router.history.back());

    const ask = await screen.findByRole("alertdialog", {
      name: "Leave without your backup codes?",
    });

    expect(screen.getByRole("list", { name: "Backup codes" })).toBeTruthy();
    fireEvent.click(within(ask).getByRole("button", { name: "Stay" }));
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
    expect(router.state.location.pathname).toBe("/two_factor/setup");
    expect(screen.getByRole("list", { name: "Backup codes" })).toBeTruthy();
    // The browser has stepped forward again, back to the codes.
    await waitFor(() => expect(window.location.pathname).toBe("/app/two_factor/setup"));

    await act(async () => router.history.back());
    fireEvent.click(
      within(
        await screen.findByRole("alertdialog", { name: "Leave without your backup codes?" }),
      ).getByRole("button", { name: "Leave without them" }),
    );
    expect(await screen.findByText("Room destination")).toBeTruthy();
    expect(router.state.location.pathname).toBe("/r/4");
  });
});

describe("the enrollment gate", () => {
  it("routes a gated request to setup and returns to the page once set up", async () => {
    await twoFactor({ enrolled: false, required: true });

    const router = await mount("/app/r/4?message=9");

    await screen.findByRole("img", { name: "QR code for your authenticator app" });
    expect(router.state.location.pathname).toBe("/two_factor/setup");
    expect(router.state.location.search).toEqual({
      return_to: "/app/r/4?message=9",
      required: true,
    });
    expect(screen.queryByRole("button", { name: "Back to settings" })).toBeNull();

    await enter("123456");
    fireEvent.click(await screen.findByRole("button", { name: "I've saved these codes" }));
    expect(leave).toHaveBeenCalledWith("/app/r/4?message=9");
  });

  it("leaves an enrolled session where it is", async () => {
    const router = await mount("/app/r/4");

    await act(() => settings.account());
    expect(router.state.location.pathname).toBe("/r/4");
  });
});

describe("the setup screen's query", () => {
  it("keeps only a return to this app", () => {
    expect(safeReturnPath("/app/r/4?message=9")).toBe("/app/r/4?message=9");
    expect(safeReturnPath("/app")).toBe("/app");

    for (const unsafe of [
      "//evil.example/app",
      "https://evil.example/app",
      "/apple",
      "/app//x",
      "/app/two_factor/setup?required=true",
    ]) {
      expect(safeReturnPath(unsafe), unsafe).toBeNull();
    }

    expect(
      parseTwoFactorSetupSearch({ return_to: "https://evil.example", required: "true" }),
    ).toEqual({ required: true });
    expect(parseTwoFactorSetupSearch({ return_to: "/app/settings/security" })).toEqual({
      return_to: "/app/settings/security",
    });
  });
});
