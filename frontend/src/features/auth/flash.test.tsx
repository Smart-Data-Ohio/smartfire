import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterAll, afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MOCK_INVITE, MOCK_JOIN_CODE } from "../../../mock/s2/join.ts";
import { MOCK_PASSWORD } from "../../../mock/s2/settings.ts";
import { MOCK_TWO_FACTOR_EMAIL } from "../../../mock/s2/sign-in.ts";
import type { SignedOutBoot } from "../../gen/SignedOutBoot.ts";
import { installMockNetwork } from "../../test/mock-network.ts";
import { pageExit } from "./auth-navigation.ts";
import { renderAuth, resetSignIn } from "./testing.tsx";

const network = installMockNetwork();

afterAll(() => network.restore());

const GOOGLE_EXPIRED = "Google sign-in expired. Try again or sign in with email and password.";

beforeEach(() => {
  vi.spyOn(pageExit, "assign").mockImplementation(() => undefined);
  vi.spyOn(pageExit, "replace").mockImplementation(() => undefined);
  // jsdom has no object URLs; the join and first-run avatar pickers don't need one here.
  URL.createObjectURL = () => "blob:preview";
  URL.revokeObjectURL = () => undefined;
});

afterEach(() => {
  vi.restoreAllMocks();
  document.getElementById("boot")?.remove();
});

/** The boot the Rust shell inlines, carrying `flash`. */
function inlineBoot(flash: SignedOutBoot["flash"], firstRunPending = false): void {
  const boot = document.createElement("script");

  const data: SignedOutBoot = {
    kind: "signedOut",
    workspace: { name: "Harbor", logoUrl: null, description: "" },
    signInMethods: { password: true, google: true, googleDomains: ["smartdata.example"] },
    firstRunPending,
    helpContact: null,
    version: "2.0.0",
    csrfToken: network.server.csrfToken(),
    flash,
  };

  boot.id = "boot";
  boot.type = "application/json";
  boot.textContent = JSON.stringify(data);
  document.body.append(boot);
}

const alert = { kind: "alert", message: GOOGLE_EXPIRED } as const;

describe("a redirect's flash", () => {
  it.each([
    ["sign-in", "/app/session/new", {}],
    ["the second step", "/app/two_factor/challenge", { pending: true }],
    ["the sign-in link", "/app/session/transfers/unknown-link", {}],
    ["first run", "/app/first_run", { firstRunPending: true }],
    ["joining", `/app/join/${MOCK_JOIN_CODE}`, {}],
    ["an invite", `/app/invite/${MOCK_INVITE}`, {}],
  ] as const)("shows an alert above the card on %s", async (_page, path, change) => {
    await resetSignIn(network, change);
    inlineBoot(alert, path === "/app/first_run");

    await renderAuth(path);

    const shown = (await screen.findByText(GOOGLE_EXPIRED)).closest<HTMLElement>(
      ".auth-view-flash",
    );

    expect(shown?.getAttribute("role")).toBe("alert");
    expect(shown?.textContent).toBe(GOOGLE_EXPIRED);
    expect(shown?.dataset.tone).toBe("danger");
    // Above the card, as the retained auth layout puts it.
    expect(shown?.nextElementSibling?.tagName).toBe("MAIN");
  });

  it("says a notice politely", async () => {
    await resetSignIn(network);
    inlineBoot({ kind: "notice", message: "Google sign-in linked to ada@smartdata.example." });

    await renderAuth("/app/session/new");

    const shown = await screen.findByRole("status");

    expect(shown.textContent).toBe("Google sign-in linked to ada@smartdata.example.");
    expect(shown.dataset.tone).toBe("success");
    expect(document.querySelector(".auth-view-flash[role='alert']")).toBeNull();
  });

  it("shows nothing when the redirect left none", async () => {
    await resetSignIn(network);
    inlineBoot(null);

    await renderAuth("/app/session/new");

    expect(await screen.findByRole("textbox", { name: "Email address" })).toBeTruthy();
    expect(document.querySelector(".auth-view-flash")).toBeNull();
  });

  it("belongs to the page load, not to the next signed-out page", async () => {
    await resetSignIn(network);
    inlineBoot(alert);

    const router = await renderAuth("/app/session/new");

    expect((await screen.findByRole("alert")).textContent).toBe(GOOGLE_EXPIRED);

    const user = userEvent.setup();

    await user.type(screen.getByRole("textbox", { name: "Email address" }), MOCK_TWO_FACTOR_EMAIL);
    await user.type(screen.getByLabelText("Password"), MOCK_PASSWORD);
    await user.click(screen.getByRole("button", { name: /^Sign in$/ }));

    expect(await screen.findByRole("heading", { level: 1, name: "Enter your code" })).toBeTruthy();
    expect(router.pathname()).toBe("/two_factor/challenge");
    expect(document.querySelector(".auth-view-flash")).toBeNull();
  });

  it("shows the flash of a boot fetched from the server, once", async () => {
    await resetSignIn(network, { alert: GOOGLE_EXPIRED });

    await renderAuth("/app/session/new");

    expect((await screen.findByRole("alert")).textContent).toBe(GOOGLE_EXPIRED);

    const again = await fetch("/api/v1/session/boot").then((reply) => reply.json());

    expect(again.flash).toBeNull();
  });
});
