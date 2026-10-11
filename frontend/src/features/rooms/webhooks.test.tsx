import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { userFixture } from "../../api/testing.ts";
import type { ChannelWebhook } from "../../gen/ChannelWebhook.ts";
import type { ChannelWebhookCredential } from "../../gen/ChannelWebhookCredential.ts";
import { initialState } from "../../store/state.ts";
import { mutations, store } from "../../store/store.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { WebhooksSection } from "./webhooks.tsx";

const ROOM = 7;

const TOKEN = "s3cr3t-token-value";

function hook(patch: Partial<ChannelWebhook> = {}): ChannelWebhook {
  return {
    id: 11,
    roomId: ROOM,
    creatorId: 1,
    name: "Deployments",
    avatarUrl: null,
    lastUsedAt: null,
    createdAt: "2026-10-03T15:00:00.000Z",
    updatedAt: "2026-10-03T15:00:00.000Z",
    ...patch,
  };
}

function credential(webhook: ChannelWebhook, token = TOKEN): ChannelWebhookCredential {
  return { webhook, token, url: `https://chat.example.test/api/webhooks/${webhook.id}/${token}` };
}

/** Everything lasting the page could have kept: the store, and both web storages. */
function kept(): string {
  const storages = [localStorage, sessionStorage].map((storage) =>
    Object.keys(storage)
      .map((key) => `${key}=${storage.getItem(key) ?? ""}`)
      .join("\n"),
  );

  return `${JSON.stringify(store.getState())}\n${storages.join("\n")}`;
}

afterEach(() => {
  vi.restoreAllMocks();
  store.setState(initialState, true);
});

describe("Webhooks section", () => {
  it("lists each webhook with who made it and when", async () => {
    mutations.mergeUsers([userFixture(1, "Priya Shah")]);
    vi.spyOn(actions.rooms, "channelWebhooks").mockResolvedValue({
      webhooks: [hook(), hook({ id: 12, name: "Alerts", lastUsedAt: "2026-10-05T09:00:00.000Z" })],
    });

    render(<WebhooksSection roomId={ROOM} open />);

    const deployments = await screen.findByRole("listitem", { name: "Deployments" });

    expect(within(deployments).getByText(/Created by Priya Shah on/)).toBeTruthy();
    expect(within(deployments).getByText(/Never used/)).toBeTruthy();
    expect(
      within(screen.getByRole("listitem", { name: "Alerts" })).getByText(/Last used/),
    ).toBeTruthy();
  });

  it("shows a new webhook's URL once, with a copy button, and forgets it when done", async () => {
    vi.spyOn(actions.rooms, "channelWebhooks").mockResolvedValue({ webhooks: [] });

    const create = vi
      .spyOn(actions.rooms, "createChannelWebhook")
      .mockResolvedValue(credential(hook()));

    const user = userEvent.setup();

    render(<WebhooksSection roomId={ROOM} open />);
    await user.type(await screen.findByLabelText("Name"), "  Deployments ");
    await user.click(screen.getByRole("button", { name: "Create webhook" }));

    expect(create).toHaveBeenCalledWith(ROOM, { name: "Deployments", avatarSignedId: null });

    const notice = await screen.findByRole("status", { name: "Deployments's webhook URL" });

    expect(within(notice).getByText(/You won't see this URL again/)).toBeTruthy();
    expect(notice.textContent).toContain(TOKEN);
    expect(kept()).not.toContain(TOKEN);

    await user.click(within(notice).getByRole("button", { name: "Copy" }));
    // user-event stands in for the clipboard.
    await waitFor(async () =>
      expect(await navigator.clipboard.readText()).toBe(
        `https://chat.example.test/api/webhooks/11/${TOKEN}`,
      ),
    );

    await user.click(within(notice).getByRole("button", { name: "Done" }));
    expect(screen.queryByRole("status", { name: /webhook URL/ })).toBeNull();
    expect(document.body.textContent).not.toContain(TOKEN);
    expect(screen.getByRole("listitem", { name: "Deployments" })).toBeTruthy();
  });

  it("drops a URL on show as soon as the settings close", async () => {
    vi.spyOn(actions.rooms, "channelWebhooks").mockResolvedValue({ webhooks: [hook()] });
    vi.spyOn(actions.rooms, "regenerateChannelWebhook").mockResolvedValue(
      credential(hook(), "fresh-token"),
    );

    const user = userEvent.setup();
    const view = render(<WebhooksSection roomId={ROOM} open />);

    await user.click(
      await screen.findByRole("button", { name: "Generate a new URL for Deployments" }),
    );
    await user.click(
      within(screen.getByRole("alertdialog", { name: "New URL for Deployments?" })).getByRole(
        "button",
        { name: "Generate new URL" },
      ),
    );
    expect(await screen.findByText(/fresh-token/)).toBeTruthy();

    view.rerender(<WebhooksSection roomId={ROOM} open={false} />);
    expect(document.body.textContent).not.toContain("fresh-token");

    view.rerender(<WebhooksSection roomId={ROOM} open />);
    expect(document.body.textContent).not.toContain("fresh-token");
  });

  it("renames, sends a test, and deletes after a confirm", async () => {
    vi.spyOn(actions.rooms, "channelWebhooks").mockResolvedValue({ webhooks: [hook()] });

    const rename = vi
      .spyOn(actions.rooms, "updateChannelWebhook")
      .mockResolvedValue(hook({ name: "Releases" }));

    const test = vi.spyOn(actions.rooms, "testChannelWebhook").mockResolvedValue(undefined);
    const remove = vi.spyOn(actions.rooms, "deleteChannelWebhook").mockResolvedValue(undefined);
    const user = userEvent.setup();

    render(<WebhooksSection roomId={ROOM} open />);
    await user.click(await screen.findByRole("button", { name: "Rename Deployments" }));

    const row = screen.getByRole("listitem", { name: "Deployments" });
    const field = within(row).getByLabelText("Name");

    await user.clear(field);
    await user.type(field, "Releases{Enter}");
    expect(rename).toHaveBeenCalledWith(ROOM, 11, { name: "Releases" });
    expect(await screen.findByRole("listitem", { name: "Releases" })).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Send a test message from Releases" }));
    expect(test).toHaveBeenCalledWith(ROOM, 11);

    await user.click(screen.getByRole("button", { name: "Delete Releases" }));
    expect(remove).not.toHaveBeenCalled();
    await user.click(
      within(screen.getByRole("alertdialog", { name: "Delete Releases?" })).getByRole("button", {
        name: "Delete webhook",
      }),
    );
    expect(remove).toHaveBeenCalledWith(ROOM, 11);
    expect(await screen.findByText("No webhooks yet.")).toBeTruthy();
  });

  it("shows the server's reason for a refused name on the field", async () => {
    vi.spyOn(actions.rooms, "channelWebhooks").mockResolvedValue({ webhooks: [] });
    vi.spyOn(actions.rooms, "createChannelWebhook").mockRejectedValue(
      new ActionError("Validation", "Name must have 1 to 80 characters", {
        name: ["must have 1 to 80 characters"],
      }),
    );

    const user = userEvent.setup();

    render(<WebhooksSection roomId={ROOM} open />);
    await user.type(await screen.findByLabelText("Name"), "x");
    await user.click(screen.getByRole("button", { name: "Create webhook" }));
    expect(await screen.findByText("Name must have 1 to 80 characters.")).toBeTruthy();
  });

  it("explains a refusal instead of the controls", async () => {
    vi.spyOn(actions.rooms, "channelWebhooks").mockRejectedValue(
      new ActionError("Forbidden", "Not allowed"),
    );

    render(<WebhooksSection roomId={ROOM} open />);
    await waitFor(() =>
      expect(screen.getByText(/administrators can manage its webhooks/)).toBeTruthy(),
    );
    expect(screen.queryByRole("button", { name: "Create webhook" })).toBeNull();
  });
});
