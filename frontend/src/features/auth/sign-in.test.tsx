import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterAll, afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MOCK_PASSWORD } from "../../../mock/s2/settings.ts";
import {
  MOCK_PLAIN_EMAIL,
  MOCK_TWO_FACTOR_EMAIL,
  SIGN_IN_REJECTION,
  SIGNED_IN_LOCATION,
} from "../../../mock/s2/sign-in.ts";
import { installMockNetwork } from "../../test/mock-network.ts";
import { FIRST_RUN_PATH, pageExit } from "./auth-navigation.ts";
import { domainSentence } from "./sign-in.tsx";
import { renderAuth, resetSignIn } from "./testing.tsx";

const network = installMockNetwork();

afterAll(() => network.restore());

let assign = vi.spyOn(pageExit, "assign");

let replace = vi.spyOn(pageExit, "replace");

beforeEach(() => {
  assign = vi.spyOn(pageExit, "assign").mockImplementation(() => undefined);
  replace = vi.spyOn(pageExit, "replace").mockImplementation(() => undefined);
});

afterEach(() => {
  vi.restoreAllMocks();
  document.getElementById("boot")?.remove();
});

const emailField = () => screen.getByRole("textbox", { name: "Email address" });

// SAFETY: the password field is an <input>.
const passwordField = () => screen.getByLabelText("Password") as HTMLInputElement;

async function signIn(email: string, password: string): Promise<void> {
  const user = userEvent.setup();

  await user.type(await screen.findByRole("textbox", { name: "Email address" }), email);
  await user.type(passwordField(), password);
  await user.click(screen.getByRole("button", { name: /^Sign in$/ }));
}

describe("branding", () => {
  it("shows the workspace the Rust shell inlined, without asking the server", async () => {
    await resetSignIn(network);

    const boot = document.createElement("script");

    boot.id = "boot";
    boot.type = "application/json";
    boot.textContent = JSON.stringify({
      kind: "signedOut",
      workspace: { name: "Harbor", logoUrl: "/logo.png", description: "Crew room.\nBe kind." },
      signInMethods: { password: true, google: false, googleDomains: [] },
      firstRunPending: false,
      helpContact: null,
      version: "2.0.0",
      csrfToken: network.server.csrfToken(),
    });
    document.body.append(boot);

    const fetches = vi.spyOn(globalThis, "fetch");

    await renderAuth("/app/session/new");

    expect(screen.getByRole("heading", { level: 1, name: "Harbor" })).toBeTruthy();
    expect(document.querySelector(".auth-view-logo")?.getAttribute("src")).toBe("/logo.png");
    expect(screen.getByText("Sign in to your workspace")).toBeTruthy();
    expect(document.querySelector(".auth-view-description")?.textContent).toBe(
      "Crew room.\nBe kind.",
    );
    // As the retained page has it, the version goes with the help contact.
    expect(screen.queryByText(/Smartfire™ version/)).toBeNull();
    expect(screen.queryByRole("button", { name: "Sign in with Google" })).toBeNull();
    expect(fetches).not.toHaveBeenCalled();
  });

  it("fetches the public boot when none was inlined, with the help contact", async () => {
    await resetSignIn(network);
    await renderAuth("/app/session/new");

    expect(await screen.findByRole("heading", { level: 1, name: "Smart Data" })).toBeTruthy();
    expect(emailField()).toBe(document.activeElement);
    expect(screen.getByRole("link", { name: MOCK_TWO_FACTOR_EMAIL }).getAttribute("href")).toBe(
      `mailto:${MOCK_TWO_FACTOR_EMAIL}`,
    );
    expect(screen.getByText("Smartfire™ version 2.0.0-mock")).toBeTruthy();
    expect(screen.getByRole("link", { name: "Privacy Policy" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Translate password" })).toBeTruthy();
  });
});

describe("Google", () => {
  it("is hidden until Google sign-in is configured", async () => {
    await resetSignIn(network, { google: false });
    await renderAuth("/app/session/new");

    await screen.findByRole("textbox", { name: "Email address" });
    expect(screen.queryByRole("button", { name: "Sign in with Google" })).toBeNull();
    expect(screen.queryByText(/Google sign-in for/)).toBeNull();
  });

  it("names the allowed domains and leaves for Google", async () => {
    await resetSignIn(network, { google: true });
    await renderAuth("/app/session/new");

    const google = await screen.findByRole("button", { name: "Sign in with Google" });

    expect(screen.getByText(/Google sign-in for @smartdata\.example accounts\./)).toBeTruthy();
    await userEvent.setup().click(google);
    await waitFor(() =>
      expect(assign).toHaveBeenCalledWith(expect.stringContaining("accounts.google.com")),
    );
  });

  it("joins domains into a sentence", () => {
    expect(domainSentence(["a.com"])).toBe("@a.com");
    expect(domainSentence(["a.com", "b.com"])).toBe("@a.com and @b.com");
    expect(domainSentence(["a.com", "b.com", "c.com"])).toBe("@a.com, @b.com, and @c.com");
  });
});

describe("password sign-in", () => {
  it("keeps the email, clears the password and says why after wrong credentials", async () => {
    await resetSignIn(network);
    await renderAuth("/app/session/new");
    await signIn(MOCK_PLAIN_EMAIL, "wrong-password");

    expect(await screen.findByText(SIGN_IN_REJECTION)).toBeTruthy();
    expect(emailField()).toHaveProperty("value", MOCK_PLAIN_EMAIL);
    expect(passwordField().value).toBe("");
    expect(passwordField().getAttribute("aria-invalid")).toBe("true");
    expect(passwordField()).toBe(document.activeElement);
    expect(assign).not.toHaveBeenCalled();
  });

  it("says the same when the sign-in is rate limited", async () => {
    await resetSignIn(network);
    await renderAuth("/app/session/new");
    await signIn("limit@smartdata.example", MOCK_PASSWORD);

    expect(await screen.findByText(SIGN_IN_REJECTION)).toBeTruthy();
    expect(passwordField().getAttribute("aria-invalid")).toBe("true");
  });

  it("leaves for the app once signed in", async () => {
    await resetSignIn(network);
    await renderAuth("/app/session/new");
    await signIn(MOCK_PLAIN_EMAIL, MOCK_PASSWORD);

    await waitFor(() => expect(assign).toHaveBeenCalledWith(SIGNED_IN_LOCATION));
  });

  it("goes on to the challenge page when a second step is required", async () => {
    await resetSignIn(network);

    const router = await renderAuth("/app/session/new");

    await signIn(MOCK_TWO_FACTOR_EMAIL, MOCK_PASSWORD);

    expect(await screen.findByRole("textbox", { name: "Authenticator code" })).toBeTruthy();
    expect(router.pathname()).toBe("/two_factor/challenge");
    expect(assign).not.toHaveBeenCalled();
  });
});

it("hands a fresh install to first run", async () => {
  await resetSignIn(network, { firstRunPending: true });
  await renderAuth("/app/session/new");

  await waitFor(() => expect(replace).toHaveBeenCalledWith(FIRST_RUN_PATH));
  expect(screen.queryByRole("textbox", { name: "Email address" })).toBeNull();
});
