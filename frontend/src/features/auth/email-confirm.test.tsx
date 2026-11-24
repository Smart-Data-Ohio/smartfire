import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterAll, afterEach, beforeEach, expect, it, vi } from "vitest";
import {
  EMAIL_TAKEN,
  INVALID_EMAIL_LINK,
  MOCK_EMAIL_SIGNED_OUT_TOKEN,
  MOCK_EMAIL_TAKEN_TOKEN,
  MOCK_EMAIL_TOKEN,
} from "../../../mock/s2/email-addresses.ts";
import { auth } from "../../sync/auth.ts";
import { installMockNetwork } from "../../test/mock-network.ts";
import { pageExit } from "./auth-navigation.ts";
import { renderAuth, resetSignIn } from "./testing.tsx";

const network = installMockNetwork();

afterAll(() => network.restore());

let assign = vi.spyOn(pageExit, "assign");

beforeEach(async () => {
  assign = vi.spyOn(pageExit, "assign").mockImplementation(() => undefined);
  await resetSignIn(network);
});

afterEach(() => vi.restoreAllMocks());

it("shows the address and links it only on Confirm, then loads the next page in full", async () => {
  const confirm = vi.spyOn(auth, "confirmEmailAddress");

  await renderAuth(`/app/email_addresses/confirm/${MOCK_EMAIL_TOKEN}`);

  expect(
    await screen.findByRole("heading", { level: 1, name: "Confirm your email address" }),
  ).toBeTruthy();
  expect(screen.getByText("extra@smartdata.example")).toBeTruthy();
  expect(screen.queryByText("You'll sign in after confirming.")).toBeNull();
  // Opening the link (a mail scanner's prefetch) links nothing.
  expect(confirm).not.toHaveBeenCalled();

  await userEvent.setup().click(screen.getByRole("button", { name: "Confirm" }));

  expect(confirm).toHaveBeenCalledWith(MOCK_EMAIL_TOKEN);
  await waitFor(() => expect(assign).toHaveBeenCalledWith("/app/settings"));
});

it("says a signed-out browser signs in after confirming", async () => {
  await renderAuth(`/app/email_addresses/confirm/${MOCK_EMAIL_SIGNED_OUT_TOKEN}`);

  expect(await screen.findByText("You'll sign in after confirming.")).toBeTruthy();
  await userEvent.setup().click(screen.getByRole("button", { name: "Confirm" }));
  await waitFor(() => expect(assign).toHaveBeenCalledWith("/app/session/new"));
});

it("says when a link is invalid or expired, without a Confirm button", async () => {
  await renderAuth("/app/email_addresses/confirm/expired-link");

  expect(await screen.findByRole("alert")).toHaveProperty("textContent", INVALID_EMAIL_LINK);
  expect(screen.queryByRole("button", { name: "Confirm" })).toBeNull();
  expect(screen.getByRole("link", { name: "Go to settings" }).getAttribute("href")).toBe(
    "/app/settings/profile",
  );
});

it("says when another account linked the address first", async () => {
  await renderAuth(`/app/email_addresses/confirm/${MOCK_EMAIL_TAKEN_TOKEN}`);
  await userEvent.setup().click(await screen.findByRole("button", { name: "Confirm" }));

  expect(await screen.findByRole("alert")).toHaveProperty("textContent", EMAIL_TAKEN);
  expect(screen.queryByRole("button", { name: "Confirm" })).toBeNull();
  expect(assign).not.toHaveBeenCalled();
});
