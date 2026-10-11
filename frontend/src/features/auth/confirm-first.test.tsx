import {
  createMemoryHistory,
  createRootRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import type { SudoMethod } from "../../gen/SudoMethod.ts";
import { mutations } from "../../store/store.ts";
import { admin } from "../../sync/admin.ts";
import { ActionError } from "../../sync/run.ts";
import { actions, confirmationGate } from "../../sync/runtime.ts";
import { settings } from "../../sync/settings.ts";
import { installMockNetwork, type MockNetwork } from "../../test/mock-network.ts";
import { removeToast, toastSnapshot } from "../../ui/toast-store.ts";
import { AdminContext } from "../admin/admin-parts.tsx";
import { AuditLogSection } from "../admin/audit-log-section.tsx";
import { DrivePicker } from "../composer/drive-picker.tsx";
import { IntegrationsSection } from "../settings/integrations-section.tsx";
import { SettingsContext } from "../settings/settings-parts.tsx";
import { PersonalSlackSection } from "../slack/personal-slack-section.tsx";
import { SlackSetupSection } from "../slack/slack-setup-section.tsx";
import { pageExit } from "./auth-navigation.ts";
import { ReauthenticateDialog } from "./reauthenticate.tsx";

/** The mock's password (mock/s2/settings.ts). */
const PASSWORD = "secret123456";

let network: MockNetwork;

/** Where the page went: a classic form posted, a page loaded, a file saved. */
let left: string[] = [];

/** Lapses the confirmation on the mock: it isn't fresh, and asks with `methods`. */
async function lapse(methods: readonly SudoMethod[] = ["password"]) {
  await network.server.handle({
    method: "POST",
    path: "/__mock/lapse-sudo",
    body: { on: true, methods: [...methods] },
    headers: { "x-csrf-token": network.server.csrfToken() },
  });
}

/** `node` under a memory router (the Slack pages link within the SPA), beside the dialog. */
async function show(node: ReactNode) {
  const root = createRootRoute({
    component: () => (
      <>
        {node}
        <ReauthenticateDialog />
      </>
    ),
  });

  const router = createRouter({
    routeTree: root,
    basepath: "/app",
    history: createMemoryHistory({ initialEntries: ["/app/settings"] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());
}

/** Confirms with the password in the dialog the action opened. */
async function confirmInDialog() {
  const dialog = await screen.findByRole("dialog", { name: "Confirm it's you" });

  await userEvent.type(within(dialog).getByLabelText("Password"), PASSWORD);
  await userEvent.click(within(dialog).getByRole("button", { name: "Confirm password" }));
  await waitFor(() =>
    expect(screen.queryByRole("dialog", { name: "Confirm it's you" })).toBeNull(),
  );
}

/** Cancels the dialog the action opened. */
async function cancelDialog() {
  const dialog = await screen.findByRole("dialog", { name: "Confirm it's you" });

  await userEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
  await waitFor(() =>
    expect(screen.queryByRole("dialog", { name: "Confirm it's you" })).toBeNull(),
  );
}

/** Each caller: how to show it, the control that leaves, and where it goes. */
const callers = [
  {
    name: "Settings › Integrations' Enable Drive previews",
    show: async () => {
      const initial = await settings.load();

      await show(
        <SettingsContext value={{ settings: initial, replace: () => {}, update: () => {} }}>
          <IntegrationsSection />
        </SettingsContext>,
      );
    },
    control: () => screen.findByRole("button", { name: "Enable Drive previews" }),
    target: "POST /google/connect features[]=drive",
  },
  {
    name: "the Drive picker's Enable Drive previews",
    show: async () => {
      // Picker config's empty 404: Drive isn't connected.
      vi.spyOn(actions.drive, "preparePicker").mockRejectedValue(
        new ActionError("NotFound", "404"),
      );
      vi.spyOn(actions.drive, "search").mockResolvedValue({ files: [] });
      await show(
        <DrivePicker
          roomId={1}
          attachedFileIds={[]}
          open
          onOpenChange={() => undefined}
          onAttach={() => undefined}
        />,
      );
    },
    control: () => screen.findByRole("button", { name: "Enable Drive previews" }),
    target: "POST /google/connect features[]=drive",
  },
  {
    name: "Slack import's Connect Slack",
    show: async () => {
      const workspace = await admin.workspace();

      await show(
        <AdminContext value={{ workspace, replace: () => {} }}>
          <SlackSetupSection />
        </AdminContext>,
      );
    },
    control: () => screen.findByRole("link", { name: /connect slack/i }),
    target: "GET /slack/oauth/start?return_to=%2Faccount%2Fslack_import",
  },
  {
    name: "a person's Connect Slack",
    show: () => show(<PersonalSlackSection />),
    control: () => screen.findByRole("link", { name: /connect slack/i }),
    target: "GET /slack/oauth/start?return_to=%2Fslack%2Fimports",
  },
  {
    name: "the audit log's Export CSV",
    show: async () => {
      const workspace = await admin.workspace();

      await show(
        <AdminContext value={{ workspace, replace: () => {} }}>
          <AuditLogSection />
        </AdminContext>,
      );
    },
    control: () => screen.findByRole("link", { name: "Export CSV" }),
    target: "SAVE /account/audit_log.csv",
  },
] as const;

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
  left = [];
  window.matchMedia = (query: string) =>
    Object.assign(new EventTarget(), {
      matches: false,
      media: query,
      onchange: null,
      addListener: () => undefined,
      removeListener: () => undefined,
    });
  vi.spyOn(HTMLFormElement.prototype, "submit").mockImplementation(function (
    this: HTMLFormElement,
  ) {
    const fields = [...new FormData(this).entries()].flatMap(([name, value]) =>
      name === "authenticity_token" ? [] : [`${name}=${String(value)}`],
    );

    left.push([`POST ${new URL(this.action).pathname}`, ...fields].join(" "));
  });
  vi.spyOn(pageExit, "assign").mockImplementation((url) => {
    left.push(`GET ${url.replace(window.location.origin, "")}`);
  });
  vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (
    this: HTMLAnchorElement,
  ) {
    if (this.hasAttribute("download")) {
      left.push(`SAVE ${new URL(this.href).pathname}`);
    }
  });
});

afterEach(() => {
  // As if back from the page it left for: the controls go again.
  window.dispatchEvent(new Event("pageshow"));
  confirmationGate.cancel();
  cleanup();
  mutations.reset();
  vi.restoreAllMocks();

  for (const record of [...toastSnapshot()]) {
    removeToast(record.id);
  }
});

describe.each(callers)("$name", (caller) => {
  it("goes at once while the confirmation is fresh", async () => {
    await caller.show();
    await userEvent.click(await caller.control());

    await waitFor(() => expect(left).toEqual([caller.target]));
    expect(screen.queryByRole("dialog", { name: "Confirm it's you" })).toBeNull();
  });

  it("asks in the dialog first when it isn't, then goes", async () => {
    await lapse();
    await caller.show();
    await userEvent.click(await caller.control());
    await confirmInDialog();

    await waitFor(() => expect(left).toEqual([caller.target]));
  });

  it("stays when the person cancels", async () => {
    await lapse();
    await caller.show();
    await userEvent.click(await caller.control());
    await cancelDialog();

    expect(left).toEqual([]);
  });
});

describe("confirming before a full-page action", () => {
  it("says to click again when Google takes the person away", async () => {
    await lapse(["password", "google"]);
    await callers[0].show();
    await userEvent.click(await callers[0].control());

    const dialog = await screen.findByRole("dialog", { name: "Confirm it's you" });

    expect(
      within(dialog).getByText(
        "Google opens in this tab. When you're back, click what you were doing again to finish.",
      ),
    ).toBeTruthy();
  });

  it("leaves a click meant for a new tab to the browser", async () => {
    await lapse();
    await callers[4].show();

    const link = await callers[4].control();

    // The browser opens it; the server's own confirmation page asks there.
    fireEvent.click(link, { ctrlKey: true });

    expect(screen.queryByRole("dialog", { name: "Confirm it's you" })).toBeNull();
    expect(left).toEqual([]);
  });

  it("goes only once for clicks after it left, while the confirmation is fresh", async () => {
    await callers[0].show();

    const button = await callers[0].control();

    await userEvent.click(button);
    await waitFor(() => expect(left).toEqual([callers[0].target]));

    // The page is on its way to Google: a second click doesn't send the form again.
    await userEvent.click(button);
    await act(() => new Promise((resolve) => setTimeout(resolve, 50)));
    expect(left).toEqual([callers[0].target]);

    // Back from the back-forward cache, it goes again.
    window.dispatchEvent(new Event("pageshow"));
    await userEvent.click(button);
    await waitFor(() => expect(left).toEqual([callers[0].target, callers[0].target]));
  });

  it("a download lets the link go again after a moment, not at once", async () => {
    await callers[4].show();

    const link = await callers[4].control();

    await userEvent.click(link);
    await waitFor(() => expect(left).toEqual([callers[4].target]));
    await userEvent.click(link);
    await act(() => new Promise((resolve) => setTimeout(resolve, 50)));
    expect(left).toEqual([callers[4].target]);

    // The page stayed: after the moment, it saves again.
    await act(() => new Promise((resolve) => setTimeout(resolve, 1000)));
    await userEvent.click(link);
    await waitFor(() => expect(left).toEqual([callers[4].target, callers[4].target]));
  });

  it("signed out meanwhile, goes on to sign in instead", async () => {
    vi.spyOn(actions.confirmation, "confirmFirst").mockResolvedValue({
      kind: "signedOut",
      location: "http://campfire.test/session/new",
    });
    await callers[0].show();
    await userEvent.click(await callers[0].control());

    await waitFor(() => expect(left).toEqual(["GET http://campfire.test/session/new"]));
    expect(toastSnapshot()).toEqual([]);
    expect(screen.queryByRole("dialog", { name: "Confirm it's you" })).toBeNull();
  });

  it("goes only once for a double click", async () => {
    await lapse();
    await callers[0].show();

    const button = await callers[0].control();

    await userEvent.dblClick(button);
    await confirmInDialog();

    await waitFor(() => expect(left).toEqual([callers[0].target]));
    expect(confirmationGate.snapshot()).toBeNull();
  });
});
