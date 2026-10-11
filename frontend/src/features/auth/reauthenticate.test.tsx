import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it } from "vitest";
import type { SudoMethod } from "../../gen/SudoMethod.ts";
import { admin } from "../../sync/admin.ts";
import { ActionError } from "../../sync/run.ts";
import { confirmationGate } from "../../sync/runtime.ts";
import { installMockNetwork, type MockNetwork } from "../../test/mock-network.ts";
import { ReauthenticateDialog } from "./reauthenticate.tsx";

/** The mock's password and authenticator code (mock/s2/settings.ts, mock/s2/account.ts). */
const PASSWORD = "secret123456";

const CODE = "123456";

let network: MockNetwork;

/** Lapses the confirmation on the mock: guarded writes answer `SudoRequired` with `methods`. */
async function lapse(methods: readonly SudoMethod[]) {
  await network.server.handle({
    method: "POST",
    path: "/__mock/lapse-sudo",
    body: { on: true, methods: [...methods] },
    headers: { "x-csrf-token": network.server.csrfToken() },
  });
}

/** Saves custom CSS (a guarded write) and waits for its confirmation dialog. */
async function heldWrite(css = "body { color: red; }") {
  const write = admin.updateCustomStyles(css);

  const outcome = write.then(
    (styles) => ({ styles, error: null }),
    (error: Error) => ({ styles: null, error }),
  );

  await screen.findByRole("dialog", { name: "Confirm it's you" });

  return outcome;
}

beforeAll(() => {
  network = installMockNetwork();

  const meta = document.createElement("meta");

  meta.name = "csrf-token";
  meta.content = network.server.csrfToken();
  document.head.append(meta);
});

afterAll(() => network.restore());

beforeEach(() => {
  window.matchMedia = (query: string) =>
    Object.assign(new EventTarget(), {
      matches: false,
      media: query,
      onchange: null,
      addListener: () => undefined,
      removeListener: () => undefined,
    });
  render(<ReauthenticateDialog />);
});

afterEach(() => {
  confirmationGate.cancel();
  cleanup();
});

describe("the confirmation dialog", () => {
  it("offers the account's methods, and Cancel fails the write unsent", async () => {
    await lapse(["password", "totp", "google"]);

    const outcome = heldWrite();
    const dialog = await screen.findByRole("dialog", { name: "Confirm it's you" });

    expect(within(dialog).getByLabelText("Password")).toBeTruthy();
    expect(within(dialog).getByLabelText("Authenticator code")).toBeTruthy();
    expect(within(dialog).getByRole("button", { name: "Confirm with Google" })).toBeTruthy();
    expect(within(dialog).getAllByText("or")).toHaveLength(2);
    // Custom CSS carries no credential: nothing warns it would be lost to Google.
    expect(within(dialog).queryByText(/isn't kept/)).toBeNull();

    await userEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));

    const { error } = await outcome;

    expect(error).toBeInstanceOf(ActionError);
    expect(error instanceof ActionError ? error.tag : null).toBe("ConfirmationCancelled");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("asks again after a wrong password, then saves once the right one is in", async () => {
    await lapse(["password"]);

    const outcome = heldWrite("main { color: blue; }");
    const dialog = await screen.findByRole("dialog", { name: "Confirm it's you" });
    const field = within(dialog).getByLabelText("Password");

    expect(within(dialog).queryByLabelText("Authenticator code")).toBeNull();
    expect(within(dialog).queryByText("or")).toBeNull();

    await userEvent.click(within(dialog).getByRole("button", { name: "Confirm password" }));
    expect(await within(dialog).findByText("Enter your password.")).toBeTruthy();

    await userEvent.type(field, "wrong");
    await userEvent.click(within(dialog).getByRole("button", { name: "Confirm password" }));
    expect(await within(dialog).findByText("Confirmation failed. Try again.")).toBeTruthy();
    expect(field).toHaveProperty("value", "");
    expect(document.activeElement).toBe(field);
    expect(field.getAttribute("aria-invalid")).toBe("true");

    await userEvent.type(field, PASSWORD);
    await userEvent.click(within(dialog).getByRole("button", { name: "Confirm password" }));

    const { styles, error } = await outcome;

    expect(error).toBeNull();
    expect(styles?.css).toBe("main { color: blue; }");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("asks again after a wrong authenticator code, and says when it's rate limited", async () => {
    await lapse(["totp"]);

    const outcome = heldWrite("p { margin: 0; }");
    const dialog = await screen.findByRole("dialog", { name: "Confirm it's you" });
    const field = within(dialog).getByLabelText("Authenticator code");

    expect(field.getAttribute("inputmode")).toBe("numeric");
    expect(field.getAttribute("autocomplete")).toBe("one-time-code");

    await userEvent.type(field, "000000");
    await userEvent.click(within(dialog).getByRole("button", { name: "Confirm code" }));
    expect(await within(dialog).findByText("Confirmation failed. Try again.")).toBeTruthy();

    await userEvent.type(field, "limit");
    await userEvent.click(within(dialog).getByRole("button", { name: "Confirm code" }));
    expect(
      await within(dialog).findByText("Too many attempts. Try again in a few minutes."),
    ).toBeTruthy();

    await userEvent.type(field, CODE);
    await userEvent.click(within(dialog).getByRole("button", { name: "Confirm code" }));

    expect((await outcome).styles?.css).toBe("p { margin: 0; }");
  });

  it("asks once for writes that arrive together, and Escape cancels them all", async () => {
    await lapse(["password"]);

    const first = heldWrite("a { color: red; }");

    const second = admin.resetJoinCode().then(
      () => null,
      (error: Error) => error,
    );

    await waitFor(() => expect(confirmationGate.snapshot()?.writes).toBe(2));
    expect(screen.getAllByRole("dialog")).toHaveLength(1);

    await userEvent.keyboard("{Escape}");

    const tags = [(await first).error, await second].map((error) =>
      error instanceof ActionError ? error.tag : null,
    );

    expect(tags).toEqual(["ConfirmationCancelled", "ConfirmationCancelled"]);
  });
});
