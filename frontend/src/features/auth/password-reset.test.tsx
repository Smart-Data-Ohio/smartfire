import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterAll, afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  BLANK_NEW_PASSWORD,
  INVALID_RESET,
  MOCK_RESET_TOKEN,
  RESET_RATE_ALERT,
} from "../../../mock/s2/sign-in.ts";
import type { SignedOutBoot } from "../../gen/SignedOutBoot.ts";
import { auth } from "../../sync/auth.ts";
import { installMockNetwork } from "../../test/mock-network.ts";
import { pageExit } from "./auth-navigation.ts";
import { RESET_UNAVAILABLE } from "./password-new.tsx";
import { renderAuth, resetSignIn } from "./testing.tsx";

const network = installMockNetwork();

afterAll(() => network.restore());

let assign = vi.spyOn(pageExit, "assign");

beforeEach(async () => {
  assign = vi.spyOn(pageExit, "assign").mockImplementation(() => undefined);
  await resetSignIn(network, { passwordReset: true });
});

afterEach(() => {
  vi.restoreAllMocks();
  document.getElementById("boot")?.remove();
});

const emailField = () => screen.getByRole("textbox", { name: "Email address" });

// SAFETY: the password field is an <input>.
const passwordField = () => screen.getByLabelText("New password") as HTMLInputElement;

describe("asking for a link", () => {
  it("fills in the address from sign in and says the same whatever the answer", async () => {
    const requests = vi.spyOn(auth, "requestPasswordReset");

    await renderAuth("/app/passwords/new?email_address=%20nobody%40x.test%20");

    expect(
      await screen.findByRole("heading", { level: 1, name: "Reset your password" }),
    ).toBeTruthy();
    expect(emailField()).toHaveProperty("value", "nobody@x.test");
    expect(emailField()).toBe(document.activeElement);

    await userEvent.setup().click(screen.getByRole("button", { name: "Send reset link" }));

    expect(await screen.findByRole("heading", { level: 1, name: "Check your email" })).toBeTruthy();
    expect(screen.getByRole("status").textContent).toContain("If nobody@x.test uses a password");
    expect(screen.getByRole("status").textContent).toContain("15 minutes");
    expect(requests).toHaveBeenCalledWith("nobody@x.test");
    expect(screen.getByRole("link", { name: "Back to sign-in" }).getAttribute("href")).toBe(
      "/app/session/new?email_address=nobody%40x.test",
    );

    await userEvent.setup().click(screen.getByRole("button", { name: "try again" }));
    expect(emailField()).toHaveProperty("value", "nobody@x.test");
  });

  it("says when requests are rate limited", async () => {
    const user = userEvent.setup();

    await renderAuth("/app/passwords/new");
    await user.type(await screen.findByRole("textbox", { name: "Email address" }), "limit@x.test");
    await user.click(screen.getByRole("button", { name: "Send reset link" }));

    expect(await screen.findByText(RESET_RATE_ALERT)).toBeTruthy();
    expect(emailField().getAttribute("aria-invalid")).toBe("true");
    expect(screen.queryByRole("heading", { name: "Check your email" })).toBeNull();
  });

  it("shows a reset-specific message when the request is refused without field errors", async () => {
    await renderAuth("/app/passwords/new?email_address=ada%40x.test");
    await screen.findByRole("textbox", { name: "Email address" });
    vi.spyOn(network.server, "handle").mockResolvedValueOnce({
      status: 403,
      json: { kind: "error", fieldErrors: {} },
    });

    await userEvent.setup().click(screen.getByRole("button", { name: "Send reset link" }));

    expect(await screen.findByText("Couldn't send the reset link. Try again.")).toBeTruthy();
  });

  it("shows the administrator view when a request returns 404 with empty field errors", async () => {
    await renderAuth("/app/passwords/new?email_address=ada%40x.test");
    await screen.findByRole("textbox", { name: "Email address" });
    vi.spyOn(network.server, "handle").mockResolvedValueOnce({
      status: 404,
      json: { kind: "error", fieldErrors: {} },
    });

    await userEvent.setup().click(screen.getByRole("button", { name: "Send reset link" }));

    expect(await screen.findByText(RESET_UNAVAILABLE)).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Send reset link" })).toBeNull();
    expect(screen.getByRole("link", { name: "Back to sign-in" })).toBeTruthy();
  });

  it("shows a rate limit message when a request returns a bare 429", async () => {
    await renderAuth("/app/passwords/new?email_address=ada%40x.test");
    await screen.findByRole("textbox", { name: "Email address" });
    vi.spyOn(globalThis, "fetch").mockResolvedValueOnce(new Response(null, { status: 429 }));

    await userEvent.setup().click(screen.getByRole("button", { name: "Send reset link" }));

    expect(await screen.findByText(RESET_RATE_ALERT)).toBeTruthy();
    expect(emailField()).toHaveProperty("value", "ada@x.test");
    expect(screen.getByRole("button", { name: "Send reset link" })).toBeTruthy();
  });

  it("points to an administrator when reset links can't be emailed", async () => {
    await resetSignIn(network, { passwordReset: false });
    await renderAuth("/app/passwords/new");

    expect(await screen.findByText(/Ask a workspace administrator/)).toBeTruthy();
    expect(screen.queryByRole("textbox", { name: "Email address" })).toBeNull();
    expect(screen.getByRole("link", { name: "Back to sign-in" })).toBeTruthy();
  });
});

describe("choosing a new password", () => {
  it("sets it and goes to sign in with a full page load", async () => {
    const user = userEvent.setup();

    await renderAuth(`/app/passwords/${MOCK_RESET_TOKEN}`);

    expect(
      await screen.findByRole("heading", { level: 1, name: "Choose a new password" }),
    ).toBeTruthy();
    expect(passwordField().getAttribute("autocomplete")).toBe("new-password");
    // The server measures bcrypt's 72 bytes; a browser's maxlength counts UTF-16 units instead.
    expect(passwordField().getAttribute("maxlength")).toBeNull();
    expect(passwordField()).toBe(document.activeElement);

    await user.type(passwordField(), "a new passphrase");
    await user.click(screen.getByRole("button", { name: "Reset password" }));

    await waitFor(() => expect(assign).toHaveBeenCalledWith("/app/session/new"));
  });

  it("shows the password's problem on the field", async () => {
    const user = userEvent.setup();

    await renderAuth(`/app/passwords/${MOCK_RESET_TOKEN}`);
    await user.type(await screen.findByLabelText("New password"), "   ");
    await user.click(screen.getByRole("button", { name: "Reset password" }));

    expect(await screen.findByText(BLANK_NEW_PASSWORD)).toBeTruthy();
    expect(passwordField().getAttribute("aria-invalid")).toBe("true");
    expect(assign).not.toHaveBeenCalled();
  });

  it("says when a link is invalid or expired and offers a new one", async () => {
    const resets = vi.spyOn(auth, "resetPassword");

    await renderAuth("/app/passwords/expired-link");

    expect(await screen.findByRole("alert")).toHaveProperty("textContent", INVALID_RESET);
    expect(screen.queryByLabelText("New password")).toBeNull();
    expect(screen.getByRole("link", { name: "Request new link" }).getAttribute("href")).toBe(
      "/app/passwords/new",
    );
    expect(screen.getByRole("link", { name: "Go to sign-in" }).getAttribute("href")).toBe(
      "/app/session/new",
    );
    expect(resets).not.toHaveBeenCalled();
  });

  it("keeps the form and typed password on a 429 with empty field errors", async () => {
    const user = userEvent.setup();

    await renderAuth(`/app/passwords/${MOCK_RESET_TOKEN}`);
    await user.type(await screen.findByLabelText("New password"), "a new passphrase");
    vi.spyOn(network.server, "handle").mockResolvedValueOnce({
      status: 429,
      json: { kind: "error", fieldErrors: {} },
    });
    await user.click(screen.getByRole("button", { name: "Reset password" }));

    expect(await screen.findByText(RESET_RATE_ALERT)).toBeTruthy();
    expect(passwordField().value).toBe("a new passphrase");
    expect(screen.queryByRole("link", { name: "Request new link" })).toBeNull();

    await user.click(screen.getByRole("button", { name: "Reset password" }));

    await waitFor(() => expect(assign).toHaveBeenCalledWith("/app/session/new"));
  });

  it("offers a new link when the link stops working before the password is set", async () => {
    const user = userEvent.setup();

    vi.spyOn(auth, "resetPassword").mockResolvedValueOnce({
      kind: "error",
      status: 400,
      fieldErrors: { token: [INVALID_RESET] },
    });
    await renderAuth(`/app/passwords/${MOCK_RESET_TOKEN}`);
    await user.type(await screen.findByLabelText("New password"), "a new passphrase");
    await user.click(screen.getByRole("button", { name: "Reset password" }));

    expect(await screen.findByRole("alert")).toHaveProperty("textContent", INVALID_RESET);
    expect(screen.queryByLabelText("New password")).toBeNull();
    expect(screen.getByRole("link", { name: "Request new link" })).toBeTruthy();
  });

  it("offers to try again when the network fails", async () => {
    const user = userEvent.setup();

    vi.spyOn(auth, "passwordReset").mockRejectedValueOnce(new Error("Network unreachable"));
    await renderAuth(`/app/passwords/${MOCK_RESET_TOKEN}`);

    expect(await screen.findByRole("alert")).toHaveProperty("textContent", "Network unreachable");
    expect(screen.queryByRole("link", { name: "Request new link" })).toBeNull();

    await user.click(screen.getByRole("button", { name: "Try again" }));
    await user.type(await screen.findByLabelText("New password"), "a new passphrase");

    vi.spyOn(auth, "resetPassword").mockRejectedValueOnce(new Error("Network unreachable"));
    await user.click(screen.getByRole("button", { name: "Reset password" }));

    expect(await screen.findByText("Network unreachable")).toBeTruthy();
    expect(passwordField().value).toBe("a new passphrase");
  });

  it("points to an administrator when reset links are turned off", async () => {
    await resetSignIn(network, { passwordReset: false });
    await renderAuth(`/app/passwords/${MOCK_RESET_TOKEN}`);

    expect(await screen.findByText(RESET_UNAVAILABLE)).toBeTruthy();
    expect(screen.queryByLabelText("New password")).toBeNull();
    expect(screen.getByRole("link", { name: "Go to sign in" })).toBeTruthy();
  });

  it("shows reset is off on a 404 after the password form has opened", async () => {
    const user = userEvent.setup();

    await renderAuth(`/app/passwords/${MOCK_RESET_TOKEN}`);
    await user.type(await screen.findByLabelText("New password"), "a new passphrase");
    vi.spyOn(network.server, "handle").mockResolvedValueOnce({
      status: 404,
      json: { kind: "error", fieldErrors: {} },
    });
    await user.click(screen.getByRole("button", { name: "Reset password" }));

    expect(await screen.findByText(RESET_UNAVAILABLE)).toBeTruthy();
    expect(screen.queryByLabelText("New password")).toBeNull();
  });

  it("offers to retry a 403 with a reset-specific error", async () => {
    const user = userEvent.setup();

    await renderAuth(`/app/passwords/${MOCK_RESET_TOKEN}`);
    await user.type(await screen.findByLabelText("New password"), "a new passphrase");
    vi.spyOn(network.server, "handle").mockResolvedValueOnce({
      status: 403,
      json: { kind: "error", fieldErrors: {} },
    });
    await user.click(screen.getByRole("button", { name: "Reset password" }));

    expect(await screen.findByRole("alert")).toHaveProperty(
      "textContent",
      "Couldn't reset your password. Try again.",
    );
    await user.click(screen.getByRole("button", { name: "Try again" }));
    expect(await screen.findByLabelText("New password")).toBeTruthy();
  });

  it("doesn't check the link when the inlined boot says reset is off", async () => {
    const checks = vi.spyOn(auth, "passwordReset");
    const boot: SignedOutBoot = await fetch("/api/v1/session/boot").then((reply) => reply.json());
    const script = document.createElement("script");

    script.id = "boot";
    script.type = "application/json";
    script.textContent = JSON.stringify({
      ...boot,
      signInMethods: { ...boot.signInMethods, passwordReset: false },
    });
    document.body.append(script);
    await renderAuth(`/app/passwords/${MOCK_RESET_TOKEN}`);

    expect(await screen.findByText(RESET_UNAVAILABLE)).toBeTruthy();
    expect(checks).not.toHaveBeenCalled();
  });
});
