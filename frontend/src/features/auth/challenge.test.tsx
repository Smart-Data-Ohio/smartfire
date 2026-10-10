import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterAll, afterEach, beforeEach, expect, it, vi } from "vitest";
import { MOCK_TOTP } from "../../../mock/s2/account.ts";
import {
  CODE_RATE_ALERT,
  MOCK_BACKUP_CODE,
  SIGNED_IN_LOCATION,
  WRONG_CODE,
} from "../../../mock/s2/sign-in.ts";
import { auth } from "../../sync/auth.ts";
import { installMockNetwork } from "../../test/mock-network.ts";
import { pageExit } from "./auth-navigation.ts";
import { renderAuth, resetSignIn } from "./testing.tsx";

const network = installMockNetwork();

afterAll(() => network.restore());

let assign = vi.spyOn(pageExit, "assign");

beforeEach(async () => {
  assign = vi.spyOn(pageExit, "assign").mockImplementation(() => undefined);
  await resetSignIn(network, { pending: true });
});

afterEach(() => vi.restoreAllMocks());

// SAFETY: the code field is an <input>.
const codeField = (name: string) => screen.getByRole("textbox", { name }) as HTMLInputElement;

async function enter(code: string): Promise<void> {
  const user = userEvent.setup();

  await user.type(codeField("Authenticator code"), code);
  await user.click(screen.getByRole("button", { name: /^Sign in$/ }));
}

it("asks for the authenticator code, focused, with remember-device off", async () => {
  await renderAuth("/app/two_factor/challenge");

  expect(screen.getByRole("heading", { level: 1, name: "Enter your code" })).toBeTruthy();
  expect(codeField("Authenticator code")).toBe(document.activeElement);
  expect(codeField("Authenticator code").getAttribute("autocomplete")).toBe("one-time-code");
  expect(screen.getByRole("checkbox", { name: "Remember this device for 30 days" })).toHaveProperty(
    "checked",
    false,
  );
});

it("clears a wrong code and says why", async () => {
  await renderAuth("/app/two_factor/challenge");
  await enter("000000");

  expect(await screen.findByText(WRONG_CODE)).toBeTruthy();
  expect(codeField("Authenticator code").value).toBe("");
  expect(codeField("Authenticator code").getAttribute("aria-invalid")).toBe("true");
  expect(codeField("Authenticator code")).toBe(document.activeElement);
  expect(assign).not.toHaveBeenCalled();
});

it("says when too many codes were tried", async () => {
  await renderAuth("/app/two_factor/challenge");
  await enter("limit");

  expect(await screen.findByText(CODE_RATE_ALERT)).toBeTruthy();
});

it("switches to a backup code and back, clearing the error", async () => {
  const user = userEvent.setup();

  await renderAuth("/app/two_factor/challenge");
  await enter("000000");
  await screen.findByText(WRONG_CODE);
  await user.click(screen.getByRole("button", { name: "Use a backup code instead" }));

  expect(codeField("Backup code")).toBe(document.activeElement);
  expect(codeField("Backup code").getAttribute("autocomplete")).toBe("off");
  expect(codeField("Backup code").getAttribute("aria-invalid")).not.toBe("true");

  await user.click(screen.getByRole("button", { name: "Use your authenticator app instead" }));
  expect(codeField("Authenticator code")).toBe(document.activeElement);
});

it("signs in with a backup code", async () => {
  const user = userEvent.setup();

  await renderAuth("/app/two_factor/challenge");
  await user.click(screen.getByRole("button", { name: "Use a backup code instead" }));
  await user.type(codeField("Backup code"), MOCK_BACKUP_CODE);
  await user.click(screen.getByRole("button", { name: /^Sign in$/ }));

  await waitFor(() => expect(assign).toHaveBeenCalledWith(SIGNED_IN_LOCATION));
});

it("sends remember-device with the code", async () => {
  const verify = vi.spyOn(auth, "verify");
  const user = userEvent.setup();

  await renderAuth("/app/two_factor/challenge");
  await user.click(screen.getByRole("checkbox", { name: "Remember this device for 30 days" }));
  await enter(MOCK_TOTP);

  await waitFor(() => expect(assign).toHaveBeenCalledWith(SIGNED_IN_LOCATION));
  expect(verify).toHaveBeenCalledWith(MOCK_TOTP, true);
});

it("sends a visitor with nothing pending back to sign in", async () => {
  await resetSignIn(network, { pending: false });

  const router = await renderAuth("/app/two_factor/challenge");

  expect(await screen.findByRole("textbox", { name: "Email address" })).toBeTruthy();
  expect(router.pathname()).toBe("/session/new");
});
