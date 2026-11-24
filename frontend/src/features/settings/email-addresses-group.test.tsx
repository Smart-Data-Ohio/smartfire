import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterAll, afterEach, beforeAll, beforeEach, expect, it, vi } from "vitest";
import { EMAIL_RATE_LIMITED, SEEDED_LINKED_ADDRESS } from "../../../mock/s2/email-addresses.ts";
import type { Settings } from "../../gen/Settings.ts";
import { mutations } from "../../store/store.ts";
import { confirmationGate } from "../../sync/runtime.ts";
import { settings } from "../../sync/settings.ts";
import { installMockNetwork, type MockNetwork } from "../../test/mock-network.ts";
import { ReauthenticateDialog } from "../auth/reauthenticate.tsx";
import { EmailAddressesGroup } from "./email-addresses-group.tsx";
import { ProfileSection } from "./profile-section.tsx";
import { SettingsContext } from "./settings-parts.tsx";

/** The mock's password (mock/s2/settings.ts). */
const PASSWORD = "secret123456";

const PRIMARY = "riel@smartdata.example";

let network: MockNetwork;

beforeAll(() => {
  network = installMockNetwork();

  const meta = document.createElement("meta");

  meta.name = "csrf-token";
  meta.content = network.server.csrfToken();
  document.head.append(meta);
});

afterAll(() => network.restore());

beforeEach(async () => {
  await fetch("/__mock/reset", { method: "POST" });
});

afterEach(() => {
  confirmationGate.cancel();
  cleanup();
  vi.restoreAllMocks();
  mutations.reset();
});

/** Lapses the confirmation on the mock: guarded writes answer `SudoRequired`. */
async function lapseSudo() {
  await network.server.handle({
    method: "POST",
    path: "/__mock/lapse-sudo",
    body: { on: true, methods: ["password"] },
    headers: { "x-csrf-token": network.server.csrfToken() },
  });
}

function renderGroup() {
  const onPrimaryChanged = vi.fn();

  render(
    <>
      <EmailAddressesGroup onPrimaryChanged={onPrimaryChanged} />
      <ReauthenticateDialog />
    </>,
  );

  return onPrimaryChanged;
}

const addressField = () => screen.getByRole("textbox", { name: "Add email address" });

it("lists the primary address, labelled, and each linked address with its actions", async () => {
  renderGroup();

  const list = await screen.findByRole("list", { name: "Your email addresses" });
  const rows = within(list).getAllByRole("listitem");

  expect(rows[0]?.textContent).toBe(`${PRIMARY}Primary`);
  expect(rows[1]?.textContent).toContain(SEEDED_LINKED_ADDRESS);
  expect(
    screen.getByRole("button", { name: `Make ${SEEDED_LINKED_ADDRESS} primary` }),
  ).toBeTruthy();
  expect(screen.getByRole("button", { name: `Remove ${SEEDED_LINKED_ADDRESS}` })).toBeTruthy();
  expect(screen.queryByText("No other addresses linked yet.")).toBeNull();
});

it("says when nothing else is linked", async () => {
  vi.spyOn(settings, "emailAddresses").mockResolvedValue({
    primary: PRIMARY,
    secondary: [],
    canAdd: true,
  });
  renderGroup();

  expect(await screen.findByText("No other addresses linked yet.")).toBeTruthy();
});

it("sends a link and says the same thing whether or not the address is free", async () => {
  const add = vi.spyOn(settings, "addEmailAddress");

  renderGroup();
  await userEvent.type(
    await screen.findByRole("textbox", { name: "Add email address" }),
    "new@x.test",
  );
  await userEvent.click(screen.getByRole("button", { name: "Send link" }));

  expect((await screen.findByRole("status")).textContent).toBe(
    "If new@x.test isn't already used by an account, we've emailed it a confirmation link. Open the link within 24 hours to finish linking.",
  );
  expect(add).toHaveBeenCalledWith("new@x.test");
  expect(addressField()).toHaveProperty("value", "");
});

it("shows the server's problem with the address on the field", async () => {
  renderGroup();
  await userEvent.type(await screen.findByRole("textbox", { name: "Add email address" }), "nope");
  await userEvent.click(screen.getByRole("button", { name: "Send link" }));

  expect(await screen.findByText("Email address isn't valid.")).toBeTruthy();
  expect(addressField().getAttribute("aria-invalid")).toBe("true");
});

it("says when adding is rate limited", async () => {
  renderGroup();
  await userEvent.type(
    await screen.findByRole("textbox", { name: "Add email address" }),
    "limit@x.test",
  );
  await userEvent.click(screen.getByRole("button", { name: "Send link" }));

  expect(await screen.findByText(EMAIL_RATE_LIMITED)).toBeTruthy();
  expect(screen.queryByRole("status")).toBeNull();
});

it("asks the person to confirm it's them first when the server wants it, then sends", async () => {
  await lapseSudo();
  renderGroup();
  await userEvent.type(
    await screen.findByRole("textbox", { name: "Add email address" }),
    "held@x.test",
  );
  await userEvent.click(screen.getByRole("button", { name: "Send link" }));

  const dialog = await screen.findByRole("dialog", { name: "Confirm it's you" });

  await userEvent.type(within(dialog).getByLabelText("Password"), PASSWORD);
  await userEvent.click(within(dialog).getByRole("button", { name: "Confirm password" }));

  expect((await screen.findByRole("status")).textContent).toContain("held@x.test");
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
});

it("hides adding when outgoing email isn't set up", async () => {
  vi.spyOn(settings, "emailAddresses").mockResolvedValue({
    primary: PRIMARY,
    secondary: [],
    canAdd: false,
  });
  renderGroup();

  expect(await screen.findByText(/can't email confirmation links/)).toBeTruthy();
  expect(screen.queryByRole("textbox", { name: "Add email address" })).toBeNull();
});

it("asks before removing an address, then removes it", async () => {
  const remove = vi.spyOn(settings, "removeEmailAddress");

  renderGroup();
  fireEvent.click(await screen.findByRole("button", { name: `Remove ${SEEDED_LINKED_ADDRESS}` }));

  const dialog = await screen.findByRole("alertdialog", { name: "Remove this email address?" });

  expect(dialog.textContent).toContain(SEEDED_LINKED_ADDRESS);

  expect(remove).not.toHaveBeenCalled();
  fireEvent.click(within(dialog).getByRole("button", { name: "Remove" }));

  expect(remove).toHaveBeenCalledWith(1);
  await waitFor(() => expect(screen.queryByText(SEEDED_LINKED_ADDRESS)).toBeNull());
  expect(await screen.findByText("No other addresses linked yet.")).toBeTruthy();
});

it("asks before making a linked address primary and releasing the old primary", async () => {
  const makePrimary = vi.spyOn(settings, "setPrimaryEmailAddress");
  const onPrimaryChanged = renderGroup();

  const button = await screen.findByRole("button", {
    name: `Make ${SEEDED_LINKED_ADDRESS} primary`,
  });

  const dialogName = `Make ${SEEDED_LINKED_ADDRESS} your primary address?`;

  fireEvent.click(button);

  const dialog = await screen.findByRole("alertdialog", { name: dialogName });

  expect(dialog.textContent).toContain(
    `Notifications will go to ${SEEDED_LINKED_ADDRESS}. ${PRIMARY} will be unlinked, and we'll email it a link to link it again.`,
  );
  expect(makePrimary).not.toHaveBeenCalled();
  fireEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
  await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
  expect(makePrimary).not.toHaveBeenCalled();
  fireEvent.click(button);
  const reopened = await screen.findByRole("alertdialog", { name: dialogName });

  fireEvent.click(within(reopened).getByRole("button", { name: "Make primary" }));

  expect(makePrimary).toHaveBeenCalledWith(1);
  await waitFor(() =>
    expect(screen.getAllByRole("listitem")[0]?.textContent).toBe(`${SEEDED_LINKED_ADDRESS}Primary`),
  );
  expect(screen.queryByText(PRIMARY)).toBeNull();
  expect(screen.getByText(/we'll email the old one a link to link it again/)).toBeTruthy();
  expect(onPrimaryChanged).toHaveBeenCalledOnce();
});

function ProfileHarness({ initial }: { readonly initial: Settings }) {
  const [value, replace] = useState(initial);

  return (
    <SettingsContext value={{ settings: value, replace, update: replace }}>
      <ProfileSection />
    </SettingsContext>
  );
}

it("moves the profile's email field to the new primary", async () => {
  render(<ProfileHarness initial={await settings.load()} />);

  const email = screen.getByRole("textbox", { name: "Email address" });

  expect(email).toHaveProperty("value", PRIMARY);
  fireEvent.click(
    await screen.findByRole("button", { name: `Make ${SEEDED_LINKED_ADDRESS} primary` }),
  );

  const dialog = await screen.findByRole("alertdialog", {
    name: `Make ${SEEDED_LINKED_ADDRESS} your primary address?`,
  });

  fireEvent.click(within(dialog).getByRole("button", { name: "Make primary" }));
  await waitFor(() => expect(email).toHaveProperty("value", SEEDED_LINKED_ADDRESS));
  expect(screen.queryByLabelText("Current password")).toBeNull();
});

it("refreshes the address list after Save profile changes the primary", async () => {
  render(<ProfileHarness initial={await settings.load()} />);
  const primary = await screen.findByRole("list", { name: "Your email addresses" });

  expect(within(primary).getByText(PRIMARY)).toBeTruthy();
  fireEvent.change(screen.getByRole("textbox", { name: "Email address" }), {
    target: { value: "saved@example.test" },
  });
  fireEvent.change(screen.getByLabelText("Current password"), { target: { value: PASSWORD } });
  fireEvent.click(screen.getByRole("button", { name: "Save profile" }));

  await waitFor(() => {
    const list = screen.getByRole("list", { name: "Your email addresses" });

    expect(within(list).getByText("saved@example.test")).toBeTruthy();
    expect(within(list).queryByText(PRIMARY)).toBeNull();
  });
});
