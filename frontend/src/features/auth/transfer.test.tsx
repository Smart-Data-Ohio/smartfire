import { act, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterAll, afterEach, beforeEach, expect, it, vi } from "vitest";
import {
  INVALID_TRANSFER,
  MOCK_TRANSFER_ID,
  MOCK_TRANSFER_TWO_FACTOR_ID,
  SIGNED_IN_LOCATION,
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
  await resetSignIn(network);
});

afterEach(() => vi.restoreAllMocks());

it("signs in with a valid link", async () => {
  await renderAuth(`/app/session/transfers/${MOCK_TRANSFER_ID}`);

  await waitFor(() => expect(assign).toHaveBeenCalledTimes(1));
  expect(assign).toHaveBeenCalledWith(SIGNED_IN_LOCATION);
});

it("says when a link is invalid or expired and offers sign in", async () => {
  await renderAuth("/app/session/transfers/expired-link");

  expect(
    await screen.findByRole("heading", { level: 1, name: "Couldn't sign you in" }),
  ).toBeTruthy();
  expect(screen.getByRole("alert").textContent).toBe(INVALID_TRANSFER);
  expect(screen.getByRole("link", { name: "Go to sign in" }).getAttribute("href")).toBe(
    "/app/session/new",
  );
  expect(screen.queryByRole("button", { name: "Try again" })).toBeNull();
  expect(assign).not.toHaveBeenCalled();
});

it("goes on to the challenge when the link's account has a second step", async () => {
  const router = await renderAuth(`/app/session/transfers/${MOCK_TRANSFER_TWO_FACTOR_ID}`);

  expect(await screen.findByRole("textbox", { name: "Authenticator code" })).toBeTruthy();
  expect(router.pathname()).toBe("/two_factor/challenge");
  expect(assign).not.toHaveBeenCalled();
});

it("doesn't send the link again when Back restores the page, but offers to", async () => {
  const transfers = vi.spyOn(auth, "transfer");

  await renderAuth(`/app/session/transfers/${MOCK_TRANSFER_ID}`);
  await waitFor(() => expect(assign).toHaveBeenCalledTimes(1));

  act(() => window.dispatchEvent(new PageTransitionEvent("pageshow", { persisted: true })));

  const again = await screen.findByRole("button", { name: "Continue" });

  expect(transfers).toHaveBeenCalledTimes(1);
  expect(assign).toHaveBeenCalledTimes(1);
  expect(screen.getByRole("link", { name: "Go to sign in" })).toBeTruthy();

  await userEvent.setup().click(again);
  await waitFor(() => expect(assign).toHaveBeenCalledTimes(2));
  expect(transfers).toHaveBeenCalledTimes(2);
});
