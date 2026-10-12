import { cleanup, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterAll, afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Workspace } from "../../gen/Workspace.ts";
import { admin } from "../../sync/admin.ts";
import { installMockNetwork } from "../../test/mock-network.ts";
import { AdminContext } from "./admin-parts.tsx";
import { InvitesSection } from "./invites-section.tsx";
import { renderInRouter } from "./testing.tsx";

const network = installMockNetwork();

afterAll(() => network.restore());

const workspace: Workspace = {
  name: "Smart Data",
  logoUrl: "/logo",
  logoStillUrl: null,
  bannerUrl: null,
  bannerStillUrl: null,
  logoAttached: false,
  joinUrl: "http://127.0.0.1/join/mock-join-code",
  canAdminister: true,
  restrictRoomCreationToAdministrators: false,
  afkPolicy: { roomId: null, timeoutMinutes: 5 },
  uploadLimitBytes: 100 * 1024 * 1024,
  stageAudienceBudget: 300,
  version: "test",
  description: "",
  vanitySlug: null,
  vanityUrl: null,
};

beforeEach(async () => {
  await fetch("/__mock/reset", { method: "POST" });
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

function renderSection(viewer: Workspace = workspace) {
  return renderInRouter(
    <AdminContext value={{ workspace: viewer, replace: () => {} }}>
      <InvitesSection />
    </AdminContext>,
    "invites",
  );
}

/** The invite rows, header row left out. */
async function rows(): Promise<HTMLElement[]> {
  const table = await screen.findByRole("table", { name: "Invites, newest first" });

  return within(table).getAllByRole("row").slice(1);
}

/** The newest invite's row. */
async function firstRow(): Promise<HTMLElement> {
  const [first] = await rows();

  if (first === undefined) throw new Error("expected an invite");

  return first;
}

/** A row's cell under `column`. */
function cell(row: HTMLElement, column: string): string {
  const index = ["Created by", "Created", "Expires", "Uses", "Status", "Actions"].indexOf(column);

  return within(row).getAllByRole("cell")[index]?.textContent ?? "";
}

describe("the invites section", () => {
  it("lists every invite with its creator, uses and state, and only a live one can be revoked", async () => {
    await renderSection();

    const [active, exhausted, expired, revoked] = await rows();

    if (!active || !exhausted || !expired || !revoked) throw new Error("expected four invites");

    expect(cell(active, "Created by")).toBe("Riel St. Amand");
    expect(cell(active, "Uses")).toBe("3 of 10");
    expect(cell(active, "Expires")).toMatch(/^in \d+ days$/);
    expect(cell(active, "Status")).toBe("Active");
    expect(within(active).getByRole("button", { name: /^Revoke/ })).toBeTruthy();

    expect(cell(exhausted, "Created by")).toBe("Priya Raman");
    expect(cell(exhausted, "Uses")).toBe("5 of 5");
    expect(cell(exhausted, "Expires")).toBe("Never");
    expect(cell(exhausted, "Status")).toBe("Used up");

    expect(cell(expired, "Uses")).toBe("2, no limit");
    expect(cell(expired, "Status")).toBe("Expired");
    expect(cell(expired, "Expires")).not.toMatch(/^in /);

    expect(cell(revoked, "Status")).toMatch(/^Revoked/);

    for (const dead of [exhausted, expired, revoked]) {
      expect(within(dead).queryByRole("button", { name: /^Revoke/ })).toBeNull();
    }
  });

  it("filters by state", async () => {
    const user = userEvent.setup();

    await renderSection();
    await rows();

    await user.click(screen.getByRole("tab", { name: "Revoked" }));
    expect((await rows()).map((row) => cell(row, "Status"))).toEqual([
      expect.stringMatching(/^Revoked/),
    ]);

    await user.click(screen.getByRole("tab", { name: "Used up" }));
    expect((await rows()).map((row) => cell(row, "Status"))).toEqual(["Used up"]);

    await user.click(screen.getByRole("tab", { name: "All" }));
    expect(await rows()).toHaveLength(4);
  });

  it("creates an invite with Discord's choices and shows its link once, to copy", async () => {
    const user = userEvent.setup();

    await renderSection();
    await rows();

    const expiry = screen.getByRole("combobox", { name: "Expire after" });
    const uses = screen.getByRole("combobox", { name: "Max number of uses" });

    expect(
      within(expiry)
        .getAllByRole("option")
        .map((option) => option.textContent),
    ).toEqual(["30 minutes", "1 hour", "6 hours", "12 hours", "1 day", "7 days", "Never"]);
    expect(expiry).toHaveProperty("value", "7d");
    expect(uses).toHaveProperty("value", "");
    expect(screen.getByText("Expires in 7 days. No limit on how many people can join with it."));

    await user.selectOptions(expiry, "1d");
    await user.selectOptions(uses, "5");
    expect(screen.getByText("Expires in 1 day. Works for 5 people.")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Create invite link" }));

    const dialog = await screen.findByRole("alertdialog", { name: "Your invite link" });
    const link = within(dialog).getByText(/^http:\/\/127\.0\.0\.1\/invite\/[0-9a-f]{64}$/);

    await user.click(within(dialog).getByRole("button", { name: "Copy link" }));
    // user-event stands in for the clipboard.
    await waitFor(async () => expect(await navigator.clipboard.readText()).toBe(link.textContent));
    expect(await within(dialog).findByRole("button", { name: "Copied" })).toBeTruthy();

    await user.click(within(dialog).getByRole("button", { name: "Done" }));
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());

    // The list has it first, without its link.
    const [first] = await rows();

    if (!first) throw new Error("expected the new invite");

    expect(cell(first, "Uses")).toBe("0 of 5");
    expect(cell(first, "Expires")).toBe("in 24 hours");
    expect(cell(first, "Status")).toBe("Active");
    expect(screen.queryByText(link.textContent ?? "")).toBeNull();
  });

  it("keeps an invite made while the list was still loading", async () => {
    const user = userEvent.setup();
    const before = await admin.invites();
    let release = () => {};

    // The list answers late, with what was there before the new invite.
    vi.spyOn(admin, "invites").mockReturnValue(
      new Promise((resolve) => {
        release = () => resolve(before);
      }),
    );

    await renderSection();
    await user.click(screen.getByRole("button", { name: "Create invite link" }));

    const dialog = await screen.findByRole("alertdialog", { name: "Your invite link" });

    await user.click(within(dialog).getByRole("button", { name: "Done" }));
    release();

    await waitFor(async () => expect(await rows()).toHaveLength(5));
    expect(cell(await firstRow(), "Uses")).toBe("0, no limit");
    expect(within(await firstRow()).getByRole("button", { name: /^Revoke/ })).toBeTruthy();
  });

  it("asks before revoking, and a revoked invite keeps its row", async () => {
    const user = userEvent.setup();

    await renderSection();

    const [active] = await rows();

    if (!active) throw new Error("expected an active invite");

    await user.click(within(active).getByRole("button", { name: /^Revoke/ }));

    const ask = await screen.findByRole("alertdialog", { name: "Revoke this invite?" });

    expect(
      within(ask).getByText(
        "Anyone who opens the link will be told it was revoked. The 3 people who joined with it stay. This can't be undone.",
      ),
    ).toBeTruthy();
    await user.click(within(ask).getByRole("button", { name: "Cancel" }));
    expect(cell(active, "Status")).toBe("Active");

    // From the keyboard.
    within(active)
      .getByRole("button", { name: /^Revoke/ })
      .focus();
    await user.keyboard("{Enter}");
    within(await screen.findByRole("alertdialog", { name: "Revoke this invite?" }))
      .getByRole("button", { name: "Revoke invite" })
      .focus();
    await user.keyboard("{Enter}");

    await waitFor(async () => expect(cell(await firstRow(), "Status")).toMatch(/^Revoked/));
    expect(within(await firstRow()).queryByRole("button", { name: /^Revoke/ })).toBeNull();
    // Its Revoke button is gone, so focus stays on its row rather than dropping to the page.
    await waitFor(async () => expect(document.activeElement).toBe(await firstRow()));
  });

  it("points to the workspace page for the shared join link", async () => {
    const user = userEvent.setup();
    const router = await renderSection();

    await user.click(
      screen.getByRole("link", {
        name: "The join link and vanity invite are on the Workspace page",
      }),
    );
    expect(router.pathname()).toBe("/admin/workspace");
  });

  it("is for administrators only", async () => {
    await renderSection({ ...workspace, canAdminister: false });

    expect(screen.getByText("Only administrators can see this page.")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Create invite link" })).toBeNull();
  });
});
