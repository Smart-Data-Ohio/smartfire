import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AdminOAuthClient } from "../../gen/AdminOAuthClient.ts";
import type { AdminOAuthClientList } from "../../gen/AdminOAuthClientList.ts";
import { oauthClients } from "../../sync/admin.ts";
import { ActionError } from "../../sync/run.ts";
import { toastSnapshot } from "../../ui/toast-store.ts";
import { ConnectedAppsGroup } from "./oauth-clients-section.tsx";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const CLAUDE: AdminOAuthClient = {
  id: 1,
  clientName: "Claude",
  clientId: "https://claude.ai/oauth/claude-code-client-metadata",
  clientHost: "claude.ai",
  liveConnections: 2,
  lastConnectedAt: "2026-10-13T12:00:00.000Z",
  blockedAt: null,
};

const GROK: AdminOAuthClient = {
  ...CLAUDE,
  id: 2,
  clientName: "Grok",
  clientId: "https://grok.com/oauth/client.json",
  clientHost: "grok.com",
  liveConnections: 0,
  blockedAt: "2026-10-12T12:00:00.000Z",
};

function mount(list: AdminOAuthClientList) {
  vi.spyOn(oauthClients, "list").mockResolvedValue(list);
  render(<ConnectedAppsGroup />);
}

describe("Connected apps", () => {
  it("lists the apps agents connected through, with their counts and blocks", async () => {
    mount({ clients: [GROK, CLAUDE], oauthAvailable: true });

    const apps = await screen.findByRole("list", { name: "Connected apps" });
    const rows = within(apps).getAllByRole("listitem");
    const grok = within(rows[0] ?? apps);
    const claude = within(rows[1] ?? apps);

    expect(rows).toHaveLength(2);
    expect(grok.getByText(/^Blocked .+ · 0 connected · last connected/)).toBeTruthy();
    expect(grok.getByRole("button", { name: "Unblock Grok" })).toBeTruthy();
    expect(claude.getByText("claude.ai")).toBeTruthy();
    expect(claude.getByText(/^2 connected · last connected/)).toBeTruthy();
  });

  it("blocks an app after asking", async () => {
    mount({ clients: [CLAUDE], oauthAvailable: true });

    const block = vi
      .spyOn(oauthClients, "setBlocked")
      .mockResolvedValue({ ...CLAUDE, blockedAt: "2026-10-13T13:00:00.000Z" });

    fireEvent.click(await screen.findByRole("button", { name: "Block Claude" }));

    const ask = await screen.findByRole("alertdialog", { name: "Block Claude?" });

    expect(within(ask).getByText(/stop working at once/)).toBeTruthy();
    expect(block).not.toHaveBeenCalled();
    fireEvent.click(within(ask).getByRole("button", { name: "Block" }));

    await waitFor(() => expect(block).toHaveBeenCalledWith(1, true));
    expect(await screen.findByRole("button", { name: "Unblock Claude" })).toBeTruthy();
    expect(toastSnapshot().some((toast) => toast.title === "Claude was blocked")).toBe(true);
  });

  it("unblocks at once, and says when a change fails", async () => {
    mount({ clients: [GROK], oauthAvailable: true });

    const unblock = vi
      .spyOn(oauthClients, "setBlocked")
      .mockRejectedValueOnce(new ActionError("Forbidden", "Administrators only"))
      .mockResolvedValueOnce({ ...GROK, blockedAt: null });

    fireEvent.click(await screen.findByRole("button", { name: "Unblock Grok" }));

    await waitFor(() =>
      expect(toastSnapshot().some((toast) => toast.title === "Couldn't unblock Grok")).toBe(true),
    );
    expect(screen.getByRole("button", { name: "Unblock Grok" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Unblock Grok" }));

    expect(await screen.findByRole("button", { name: "Block Grok" })).toBeTruthy();
    expect(unblock).toHaveBeenLastCalledWith(2, false);
  });

  it("says none have connected yet while OAuth is on", async () => {
    mount({ clients: [], oauthAvailable: true });

    expect(await screen.findByText("No apps have connected yet.")).toBeTruthy();
  });

  it("hides itself without OAuth when nothing ever connected", async () => {
    mount({ clients: [], oauthAvailable: false });

    await waitFor(() => expect(oauthClients.list).toHaveBeenCalled());
    await waitFor(() =>
      expect(screen.queryByRole("heading", { name: "Connected apps" })).toBeNull(),
    );
  });
});
