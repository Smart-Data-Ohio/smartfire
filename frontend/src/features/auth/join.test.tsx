import { act, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterAll, afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  INVITE_REASONS,
  MOCK_DEAD_INVITES,
  MOCK_INVITE,
  MOCK_JOIN_CODE,
  MOCK_LAST_INVITE,
} from "../../../mock/s2/join.ts";
import { MOCK_PLAIN_EMAIL, SIGNED_IN_LOCATION } from "../../../mock/s2/sign-in.ts";
import { auth } from "../../sync/auth.ts";
import { installMockNetwork } from "../../test/mock-network.ts";
import { pageExit } from "./auth-navigation.ts";
import { renderAuth, resetSignIn } from "./testing.tsx";

const network = installMockNetwork();

afterAll(() => network.restore());

let assign = vi.spyOn(pageExit, "assign");

const objectUrls = { created: 0 };

beforeEach(async () => {
  assign = vi.spyOn(pageExit, "assign").mockImplementation(() => undefined);
  objectUrls.created = 0;

  // jsdom has no object URLs; the preview only needs one to point its image at.
  URL.createObjectURL = () => {
    objectUrls.created += 1;

    return `blob:preview-${objectUrls.created}`;
  };

  URL.revokeObjectURL = () => undefined;

  await resetSignIn(network);
});

afterEach(() => {
  vi.restoreAllMocks();
});

const field = (name: string) => screen.getByRole("textbox", { name });

// SAFETY: the password field is an <input>.
const passwordField = () => screen.getByLabelText("Password") as HTMLInputElement;

// SAFETY: the avatar picker's file input is an <input>.
const avatarInput = () => screen.getByLabelText(/avatar/) as HTMLInputElement;

async function fill(name: string, email: string, password: string): Promise<void> {
  const user = userEvent.setup();

  await user.type(await screen.findByRole("textbox", { name: "Name" }), name);
  await user.type(field("Email address"), email);
  await user.type(passwordField(), password);
}

const createButton = () => screen.getByRole("button", { name: "Create account" });

const create = () => userEvent.setup().click(createButton());

describe("the join code's page", () => {
  it("shows the workspace, the form and the way to sign in", async () => {
    await renderAuth(`/app/join/${MOCK_JOIN_CODE}`);

    expect(await screen.findByRole("heading", { level: 1, name: "Join Smart Data" })).toBeTruthy();
    expect(screen.getByText("Create your account to start chatting.")).toBeTruthy();
    expect(field("Name")).toBe(document.activeElement);
    expect(field("Email address").getAttribute("autocomplete")).toBe("username");
    expect(passwordField().getAttribute("autocomplete")).toBe("new-password");
    expect(passwordField().maxLength).toBe(72);
    expect(avatarInput().accept).toBe("image/*");
    expect(screen.getByRole("button", { name: "Translate name" })).toBeTruthy();
    expect(screen.getByRole("link", { name: "Sign in" }).getAttribute("href")).toBe(
      "/app/session/new",
    );
    expect(screen.getByText("Smartfire™ version 2.0.0-mock")).toBeTruthy();
    // The join page names only whom to ask, as the retained page does.
    expect(screen.queryByRole("link", { name: "Privacy Policy" })).toBeNull();
  });

  it("says which fields are missing without sending anything", async () => {
    const joins = vi.spyOn(auth, "join");

    await renderAuth(`/app/join/${MOCK_JOIN_CODE}`);
    await screen.findByRole("textbox", { name: "Name" });
    await create();

    expect(screen.getByText("Enter your name.")).toBeTruthy();
    expect(screen.getByText("Enter your email address.")).toBeTruthy();
    expect(screen.getByText("Enter a password.")).toBeTruthy();
    expect(field("Name").getAttribute("aria-invalid")).toBe("true");
    expect(field("Name")).toBe(document.activeElement);

    await userEvent.setup().type(field("Email address"), "not-an-address");
    await create();

    expect(screen.getByText("Enter an email address like you@example.com.")).toBeTruthy();
    expect(joins).not.toHaveBeenCalled();

    // Typing in a field clears its error.
    await userEvent.setup().type(field("Name"), "Ada");
    expect(field("Name").getAttribute("aria-invalid")).not.toBe("true");
  });

  it("previews the chosen avatar, sends it and enters the app", async () => {
    await renderAuth(`/app/join/${MOCK_JOIN_CODE}`);
    await fill("Ada Lovelace", "ada@smartdata.example", "secret123456");

    const picture = new File([new Uint8Array([137, 80, 78, 71])], "me.png", {
      type: "image/png",
    });

    await userEvent.setup().upload(avatarInput(), picture);

    expect(document.querySelector(".auth-view-avatar-preview")?.getAttribute("src")).toBe(
      "blob:preview-1",
    );
    expect(screen.getByText("Change avatar, me.png chosen")).toBeTruthy();

    await create();

    await waitFor(() => expect(assign).toHaveBeenCalledWith(SIGNED_IN_LOCATION));
    expect(network.server.joined()).toEqual([
      { name: "Ada Lovelace", emailAddress: "ada@smartdata.example", avatar: "me.png" },
    ]);
    // Busy stays on while the page leaves, so a second press can't create it twice.
    expect(createButton().getAttribute("aria-busy")).toBe("true");
  });

  it("sends an address that has an account to sign in with it filled in", async () => {
    const router = await renderAuth(`/app/join/${MOCK_JOIN_CODE}`);

    await fill("Theo", MOCK_PLAIN_EMAIL, "secret123456");
    await create();

    expect(await screen.findByRole("heading", { level: 1, name: "Smart Data" })).toBeTruthy();
    expect(router.pathname()).toBe("/session/new");
    expect(field("Email address")).toHaveProperty("value", MOCK_PLAIN_EMAIL);
    expect(network.server.joined()).toEqual([]);
    expect(assign).not.toHaveBeenCalled();
  });

  it("says a wrong code isn't a join link", async () => {
    await renderAuth("/app/join/wrong-code");

    expect(await screen.findByText("This join link isn't valid.")).toBeTruthy();
    expect(screen.queryByRole("textbox", { name: "Name" })).toBeNull();
    expect(screen.getByRole("link", { name: "Sign in" })).toBeTruthy();
  });

  it("can be used again once Back restores it, without sending anything by itself", async () => {
    const joins = vi.spyOn(auth, "join");

    await renderAuth(`/app/join/${MOCK_JOIN_CODE}`);
    await fill("Ada", "ada@smartdata.example", "secret123456");
    await create();
    await waitFor(() => expect(assign).toHaveBeenCalledTimes(1));

    act(() => window.dispatchEvent(new PageTransitionEvent("pageshow", { persisted: true })));

    expect(createButton().getAttribute("aria-busy")).toBeNull();
    expect(joins).toHaveBeenCalledTimes(1);
    expect(network.server.joined()).toHaveLength(1);
  });
});

describe("an invite's page", () => {
  it("joins with a live invite", async () => {
    await renderAuth(`/app/invite/${MOCK_INVITE}`);
    await fill("Grace", "grace@smartdata.example", "secret123456");
    await create();

    await waitFor(() => expect(assign).toHaveBeenCalledWith(SIGNED_IN_LOCATION));
    expect(network.server.joined()).toEqual([
      { name: "Grace", emailAddress: "grace@smartdata.example", avatar: null },
    ]);
  });

  it.each([
    ["expired", MOCK_DEAD_INVITES.expired],
    ["exhausted", MOCK_DEAD_INVITES.exhausted],
    ["revoked", MOCK_DEAD_INVITES.revoked],
    ["unknown", "never-issued"],
  ] as const)("says a %s invite can't be used", async (refusal, token) => {
    await renderAuth(`/app/invite/${token}`);

    expect(await screen.findByText("This invite is no longer valid.")).toBeTruthy();
    expect(
      screen.getByText(
        `${INVITE_REASONS[refusal]} Ask a workspace administrator for a new invite.`,
      ),
    ).toBeTruthy();
    expect(screen.getByRole("heading", { level: 1, name: "Join Smart Data" })).toBeTruthy();
    expect(screen.queryByRole("textbox", { name: "Name" })).toBeNull();
    expect(screen.getByRole("link", { name: "Sign in" })).toBeTruthy();
  });

  it("says so when its last use is taken while the form is open", async () => {
    await renderAuth(`/app/invite/${MOCK_LAST_INVITE}`);
    await fill("Late", "late@smartdata.example", "secret123456");

    // Someone else takes the last use first.
    await fetch(`/api/v1/invite/${MOCK_LAST_INVITE}`, {
      method: "POST",
      headers: { "Content-Type": "application/json", "X-CSRF-Token": network.server.csrfToken() },
      body: JSON.stringify({
        name: "Early",
        emailAddress: "early@smartdata.example",
        password: "secret123456",
      }),
    });
    await create();

    expect(await screen.findByText("This invite is no longer valid.")).toBeTruthy();
    expect(screen.getByText(/All its uses have been taken\./)).toBeTruthy();
    expect(network.server.joined().map((person) => person.name)).toEqual(["Early"]);
    expect(assign).not.toHaveBeenCalled();
  });
});
