import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import type { SudoMethod } from "../../gen/SudoMethod.ts";
import type { SudoRetry } from "../../gen/SudoRetry.ts";
import { mutations } from "../../store/store.ts";
import { actions, type SudoHere, type SudoStatus } from "../../sync/runtime.ts";
import { installMockNetwork, type MockNetwork } from "../../test/mock-network.ts";
import { removeToast, toastSnapshot } from "../../ui/toast-store.ts";
import { pageExit } from "./auth-navigation.ts";
import { SudoNew } from "./sudo-new.tsx";

/** The mock's password and authenticator code (mock/s2/settings.ts, mock/s2/account.ts). */
const PASSWORD = "secret123456";

const CODE = "123456";

/** A page the classic gate held: the audit log export, asked for in full. */
const exportRetry: SudoRetry = {
  method: "GET",
  path: "/account/audit_log.csv?action=sign_in",
  returnTo: "/app/settings/audit-log",
};

/** A form it held: connecting Google. */
const connectRetry: SudoRetry = {
  method: "POST",
  path: "/google/connect",
  returnTo: "/app/settings/integrations",
};

let network: MockNetwork;

/** The router `open` made, to leave the page as Back would. */
let router: ReturnType<typeof createRouter>;

/** Sets the mock's confirmation: lapsed (asking with `methods`) or fresh, and what it held. */
async function sudo(
  lapsed: boolean,
  retry: SudoRetry | null,
  methods: SudoMethod[] = ["password"],
) {
  await network.server.handle({
    method: "POST",
    path: "/__mock/lapse-sudo",
    body: { on: lapsed, methods, retry },
    headers: { "x-csrf-token": network.server.csrfToken() },
  });
}

/** Renders `/app/sudo/new` among the pages it can go back to; where the router is (under `/app`). */
async function open() {
  const root = createRootRoute({ component: Outlet });

  const page = (path: string) =>
    createRoute({ getParentRoute: () => root, path, component: () => <p>Page {path}</p> });

  router = createRouter({
    routeTree: root.addChildren([
      createRoute({ getParentRoute: () => root, path: "sudo/new", component: SudoNew }),
      page("/"),
      page("settings/integrations"),
      page("settings/audit-log"),
    ]),
    basepath: "/app",
    history: createMemoryHistory({ initialEntries: ["/app/sudo/new"] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return () => router.state.location.pathname;
}

const toastTitles = () => toastSnapshot().map((record) => record.title);

let assign = vi.spyOn(pageExit, "assign");

beforeAll(() => {
  network = installMockNetwork();

  const meta = document.createElement("meta");

  meta.name = "csrf-token";
  meta.content = network.server.csrfToken();
  document.head.append(meta);
});

afterAll(() => network.restore());

beforeEach(async () => {
  await network.server.handle({ method: "POST", path: "/__mock/reset" });
  assign = vi.spyOn(pageExit, "assign").mockImplementation(() => undefined);
});

afterEach(() => {
  cleanup();
  mutations.reset();
  vi.restoreAllMocks();

  for (const record of [...toastSnapshot()]) {
    removeToast(record.id);
  }
});

describe("/app/sudo/new", () => {
  it("asks with the account's methods, focused on the first, as a page", async () => {
    await sudo(true, connectRetry, ["password", "totp", "google"]);
    await open();

    expect(await screen.findByRole("heading", { level: 1, name: "Confirm it's you" })).toBeTruthy();
    expect(document.title).toBe("Confirm it's you · Smartfire");
    await waitFor(() => expect(document.activeElement).toBe(screen.getByLabelText("Password")));
    expect(screen.getByLabelText("Authenticator code")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Confirm with Google" })).toBeTruthy();
    expect(screen.getAllByText("or")).toHaveLength(2);
    expect(screen.getByRole("button", { name: "Cancel" })).toBeTruthy();
  });

  it("after a password, goes back to make the change again, never sending it", async () => {
    await sudo(true, connectRetry);

    const pathname = await open();
    const field = await screen.findByLabelText("Password");

    await userEvent.type(field, "wrong");
    await userEvent.click(screen.getByRole("button", { name: "Confirm password" }));
    expect(await screen.findByText("Confirmation failed. Try again.")).toBeTruthy();
    expect(field).toHaveProperty("value", "");
    expect(field.getAttribute("aria-invalid")).toBe("true");

    await userEvent.type(field, PASSWORD);
    await userEvent.click(screen.getByRole("button", { name: "Confirm password" }));

    expect(await screen.findByText("Page settings/integrations")).toBeTruthy();
    expect(pathname()).toBe("/settings/integrations");
    expect(toastTitles()).toEqual(["Confirmed. Try that again."]);
    expect(assign).not.toHaveBeenCalled();
  });

  it("after a code, loads the page the server held again", async () => {
    await sudo(true, exportRetry, ["totp"]);

    const pathname = await open();
    const field = await screen.findByLabelText("Authenticator code");

    await waitFor(() => expect(document.activeElement).toBe(field));
    expect(screen.queryByLabelText("Password")).toBeNull();
    await userEvent.type(field, CODE);
    await userEvent.click(screen.getByRole("button", { name: "Confirm code" }));

    await waitFor(() => expect(assign).toHaveBeenCalledWith(exportRetry.path));
    // A download leaves the page where it was: it went back first.
    await waitFor(() => expect(pathname()).toBe("/settings/audit-log"));
  });

  it("with Google, leaves with nothing kept, for the continue page to finish", async () => {
    await sudo(true, connectRetry, ["google"]);
    await open();

    await userEvent.click(await screen.findByRole("button", { name: "Confirm with Google" }));

    await waitFor(() => expect(assign).toHaveBeenCalledWith("/app/sudo/continue"));
    expect(sessionStorage.length).toBe(0);
  });

  it("already confirmed, goes straight on to the page the server held", async () => {
    await sudo(false, exportRetry);
    await open();

    await waitFor(() => expect(assign).toHaveBeenCalledWith(exportRetry.path));
    expect(screen.queryByLabelText("Password")).toBeNull();
  });

  it("already confirmed with nothing held, goes home", async () => {
    await sudo(false, null);

    const pathname = await open();

    expect(await screen.findByText("Page /")).toBeTruthy();
    expect(pathname()).toBe("/");
    expect(assign).not.toHaveBeenCalled();
  });

  it("Cancel goes back to where the request came from", async () => {
    await sudo(true, connectRetry);

    const pathname = await open();

    await screen.findByLabelText("Password");
    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(await screen.findByText("Page settings/integrations")).toBeTruthy();
    expect(pathname()).toBe("/settings/integrations");
    expect(toastTitles()).toEqual([]);
  });

  it("Cancel with nothing held goes home", async () => {
    await sudo(true, null);

    const pathname = await open();

    await screen.findByLabelText("Password");
    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(await screen.findByText("Page /")).toBeTruthy();
    expect(pathname()).toBe("/");
  });

  it("signed out (the server's 200 sign-in hand-off), goes on to sign in", async () => {
    vi.spyOn(actions.confirmation, "status").mockResolvedValue({
      kind: "navigate",
      location: "http://campfire.test/session/new",
    });
    await open();

    await waitFor(() => expect(assign).toHaveBeenCalledWith("http://campfire.test/session/new"));
    expect(screen.queryByRole("heading", { name: "Confirm it's you" })).toBeNull();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("signed out meanwhile, a password goes on to sign in", async () => {
    await sudo(true, connectRetry);
    vi.spyOn(actions.confirmation, "confirmHere").mockResolvedValue({
      kind: "signedOut",
      location: "http://campfire.test/session/new",
    });
    await open();

    await userEvent.type(await screen.findByLabelText("Password"), PASSWORD);
    await userEvent.click(screen.getByRole("button", { name: "Confirm password" }));

    await waitFor(() => expect(assign).toHaveBeenCalledWith("http://campfire.test/session/new"));
  });

  it("left while the confirmation is on its way, the answer acts on nothing", async () => {
    await sudo(true, exportRetry);

    let answer: (reply: SudoHere) => void = () => undefined;

    vi.spyOn(actions.confirmation, "confirmHere").mockReturnValue(
      new Promise((resolve) => {
        answer = resolve;
      }),
    );

    const pathname = await open();

    await userEvent.type(await screen.findByLabelText("Password"), PASSWORD);
    await userEvent.click(screen.getByRole("button", { name: "Confirm password" }));
    await act(() => router.navigate({ to: "/" }));
    expect(await screen.findByText("Page /")).toBeTruthy();

    await act(async () => answer({ kind: "confirmed", retry: exportRetry }));

    // No download, and no going back to the audit log.
    expect(assign).not.toHaveBeenCalled();
    expect(pathname()).toBe("/");
    expect(toastTitles()).toEqual([]);
  });

  it("left while the first read is on its way, the answer acts on nothing", async () => {
    await sudo(false, exportRetry);

    let answer: (status: SudoStatus) => void = () => undefined;

    vi.spyOn(actions.confirmation, "status").mockReturnValue(
      new Promise((resolve) => {
        answer = resolve;
      }),
    );

    const pathname = await open();

    await act(() => router.navigate({ to: "/" }));
    expect(await screen.findByText("Page /")).toBeTruthy();

    await act(async () => answer({ kind: "confirmed", retry: exportRetry, session: 1 }));

    expect(assign).not.toHaveBeenCalled();
    expect(pathname()).toBe("/");
  });

  it("says when the server can't be asked, and tries again", async () => {
    const status = vi
      .spyOn(actions.confirmation, "status")
      .mockResolvedValueOnce({ kind: "unreachable", message: "The server didn't answer." });

    await sudo(true, null);
    await open();

    expect((await screen.findByRole("alert")).textContent).toContain("The server didn't answer.");
    status.mockRestore();
    await userEvent.click(screen.getByRole("button", { name: "Try again" }));
    expect(await screen.findByLabelText("Password")).toBeTruthy();
  });
});
