import { act, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterAll, afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { field, type Json, parseJson } from "../../../mock/json.ts";
import { SIGNED_IN_LOCATION, UNREADABLE_SUBMISSION } from "../../../mock/s2/sign-in.ts";
import { installMockNetwork } from "../../test/mock-network.ts";
import { pageExit } from "./auth-navigation.ts";
import { renderAuth, resetSignIn } from "./testing.tsx";

const network = installMockNetwork();

afterAll(() => network.restore());

let assign = vi.spyOn(pageExit, "assign");

let replace = vi.spyOn(pageExit, "replace");

// jsdom has no object URLs; the picker only needs one to show the chosen picture.
let created: string[] = [];

let revoked: string[] = [];

beforeEach(() => {
  assign = vi.spyOn(pageExit, "assign").mockImplementation(() => undefined);
  replace = vi.spyOn(pageExit, "replace").mockImplementation(() => undefined);
  created = [];
  revoked = [];
  Object.assign(URL, {
    createObjectURL: (file: File) => {
      const url = `blob:${file.name}`;

      created.push(url);

      return url;
    },
    revokeObjectURL: (url: string) => revoked.push(url),
  });
});

afterEach(() => {
  vi.restoreAllMocks();
  document.getElementById("boot")?.remove();
});

const nameField = () => screen.getByRole("textbox", { name: "Name" });

const emailField = () => screen.getByRole("textbox", { name: "Email address" });

// SAFETY: the password field is an <input>.
const passwordField = () => screen.getByLabelText("Password") as HTMLInputElement;

// SAFETY: the avatar field is an <input type="file">.
const avatarInput = () => screen.getByLabelText("Add your avatar") as HTMLInputElement;

const createButton = () => screen.getByRole("button", { name: /^Create account$/ });

/** The last first-run submission as the mock received it. */
async function received(): Promise<Json | undefined> {
  return field(parseJson(await (await fetch("/__mock/state")).text()), "firstRun");
}

async function fill(name: string): Promise<ReturnType<typeof userEvent.setup>> {
  const user = userEvent.setup();

  await user.type(await screen.findByRole("textbox", { name: "Name" }), name);
  await user.type(emailField(), "ada@example.com");
  await user.type(passwordField(), "secret123456");

  return user;
}

function inlineBoot(firstRunPending: boolean): void {
  const boot = document.createElement("script");

  boot.id = "boot";
  boot.type = "application/json";
  boot.textContent = JSON.stringify({
    kind: "signedOut",
    workspace: { name: null, logoUrl: null, description: "" },
    signInMethods: { password: true, google: false, googleDomains: [] },
    firstRunPending,
    helpContact: null,
    version: "2.0.0",
    csrfToken: network.server.csrfToken(),
  });
  document.body.append(boot);
}

describe("opening", () => {
  it("asks the server when the shell inlined nothing, then draws the retained form", async () => {
    await resetSignIn(network, { firstRunPending: true });
    await renderAuth("/app/first_run");

    expect(await screen.findByRole("heading", { level: 1, name: "Set up Smartfire" })).toBeTruthy();
    expect(
      screen.getByText("Create the first account. It administers the workspace."),
    ).toBeTruthy();
    expect(document.title).toBe("Set up Smartfire");
    expect(nameField()).toBe(document.activeElement);
    expect(nameField()).toHaveProperty("required", true);
    expect(emailField().getAttribute("autocomplete")).toBe("username");
    expect(passwordField().getAttribute("autocomplete")).toBe("new-password");
    expect(passwordField().maxLength).toBe(72);
    expect(avatarInput().accept).toBe("image/*");
    expect(avatarInput().required).toBe(false);
    expect(screen.getByText("Optional")).toBeTruthy();

    for (const field of ["name", "email address", "password"]) {
      expect(screen.getByRole("button", { name: `Translate ${field}` })).toBeTruthy();
    }
  });

  it("opens at once on the boot the Rust shell inlined, without asking the server", async () => {
    // The server would say the workspace exists (and send the page home) if it were asked.
    await resetSignIn(network, { firstRunPending: false });
    inlineBoot(true);
    await renderAuth("/app/first_run");

    expect(nameField()).toBe(document.activeElement);
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(replace).not.toHaveBeenCalled();
  });

  it("goes home once the workspace exists, as the retained page redirects", async () => {
    await resetSignIn(network, { firstRunPending: false });
    await renderAuth("/app/first_run");

    await waitFor(() => expect(replace).toHaveBeenCalledWith("/"));
    expect(screen.queryByRole("textbox", { name: "Name" })).toBeNull();
  });

  it("says when the server can't be reached, and tries again", async () => {
    await resetSignIn(network, { firstRunPending: true, firstRunUnavailable: true });
    await renderAuth("/app/first_run");

    const retry = await screen.findByRole("button", { name: "Try again" });

    expect(screen.getByRole("alert").textContent).not.toBe("");
    expect(screen.queryByRole("textbox", { name: "Name" })).toBeNull();
    await userEvent.setup().click(retry);
    expect(await screen.findByRole("textbox", { name: "Name" })).toBeTruthy();
  });
});

describe("setting up", () => {
  it("creates the administrator and enters the app", async () => {
    await resetSignIn(network, { firstRunPending: true });
    await renderAuth("/app/first_run");

    const user = await fill("Ada");

    await user.click(createButton());

    await waitFor(() => expect(assign).toHaveBeenCalledWith(SIGNED_IN_LOCATION));
    expect(await received()).toEqual({
      name: "Ada",
      emailAddress: "ada@example.com",
      password: "secret123456",
    });
    // The button stays busy while the page leaves, so a second press sends nothing.
    expect(createButton().getAttribute("aria-busy")).toBe("true");
  });

  it("previews the chosen avatar and sends it with the form", async () => {
    await resetSignIn(network, { firstRunPending: true });
    await renderAuth("/app/first_run");

    const user = await fill("Ada");
    const picture = new File([new Uint8Array([137, 80, 78, 71])], "ada.png", { type: "image/png" });

    await user.upload(avatarInput(), picture);

    expect(document.querySelector(".auth-view-avatar-preview")?.getAttribute("src")).toBe(
      "blob:ada.png",
    );
    expect(screen.getByText("ada.png")).toBeTruthy();

    await user.click(createButton());
    await waitFor(() => expect(assign).toHaveBeenCalledWith(SIGNED_IN_LOCATION));

    const form = await received();

    expect(form).toEqual({
      submission: JSON.stringify({
        name: "Ada",
        emailAddress: "ada@example.com",
        password: "secret123456",
      }),
      avatar: { filename: "ada.png", contentType: "image/png", byteSize: 4 },
    });
  });

  it("revokes a picture's preview when another is chosen", async () => {
    await resetSignIn(network, { firstRunPending: true });
    await renderAuth("/app/first_run");

    const user = userEvent.setup();

    await screen.findByRole("textbox", { name: "Name" });
    await user.upload(avatarInput(), new File(["a"], "one.png", { type: "image/png" }));
    await user.upload(avatarInput(), new File(["b"], "two.png", { type: "image/png" }));

    expect(created).toEqual(["blob:one.png", "blob:two.png"]);
    expect(revoked).toEqual(["blob:one.png"]);
  });

  it("shows a refusal in its kept line, and the form can be sent again", async () => {
    await resetSignIn(network, { firstRunPending: true });
    await renderAuth("/app/first_run");

    const user = await fill("fail");
    const alert = screen.getByRole("alert");

    expect(alert.textContent).toBe("");
    await user.click(createButton());

    await waitFor(() => expect(alert.textContent).toBe(UNREADABLE_SUBMISSION));
    expect(assign).not.toHaveBeenCalled();
    expect(createButton().getAttribute("aria-busy")).toBeNull();
    // Nothing typed is lost.
    expect(nameField()).toHaveProperty("value", "fail");
    expect(passwordField().value).toBe("secret123456");

    await user.clear(nameField());
    await user.type(nameField(), "Ada");
    await user.click(createButton());
    await waitFor(() => expect(assign).toHaveBeenCalledWith(SIGNED_IN_LOCATION));
  });

  it("goes home when the workspace was set up meanwhile", async () => {
    await resetSignIn(network, { firstRunPending: true });
    await renderAuth("/app/first_run");

    const user = await fill("Ada");

    await resetSignIn(network, { firstRunPending: false });
    await user.click(createButton());

    await waitFor(() => expect(assign).toHaveBeenCalledWith("/"));
  });

  it("can be used again after Back from the app", async () => {
    await resetSignIn(network, { firstRunPending: true });
    await renderAuth("/app/first_run");

    const user = await fill("Ada");

    await user.click(createButton());
    await waitFor(() => expect(assign).toHaveBeenCalled());
    act(() => window.dispatchEvent(new PageTransitionEvent("pageshow", { persisted: true })));
    expect(createButton().getAttribute("aria-busy")).toBeNull();
  });
});
