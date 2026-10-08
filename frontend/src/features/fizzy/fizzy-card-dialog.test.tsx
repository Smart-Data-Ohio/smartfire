import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { messageFixture } from "../../api/testing.ts";
import type { CreatedFizzyCard } from "../../gen/CreatedFizzyCard.ts";
import type { CreateFizzyCard } from "../../gen/CreateFizzyCard.ts";
import type { FizzyMessageCardForm } from "../../gen/FizzyMessageCardForm.ts";
import { ActionError } from "../../sync/run.ts";
import FizzyCardDialog from "./fizzy-card-dialog.tsx";
import type { CreateCard, FizzyMessageScope, ReadForm } from "./fizzy-card-model.ts";

const scope: FizzyMessageScope = { roomId: 1, threadId: null, messageId: 2 };

const form: FizzyMessageCardForm = {
  connected: true,
  boards: [
    { id: "engineering", name: "Engineering" },
    { id: "support", name: "Support" },
  ],
  title: "Signups dipped in week 3",
  description: "Signups dipped in week 3\n\nSource: https://smartfire.test/rooms/1/@2",
  excerpt: "Signups dipped in week 3",
  authorName: "Maya",
  roomDisplayName: "general",
  fizzyUserName: "Riel",
  accountName: "Smart Data",
};

const created: CreatedFizzyCard = {
  number: "580",
  url: "https://fizzy.test/cards/580",
  message: messageFixture(3, 1, { replyToMessageId: 2 }),
  notice: "Fizzy card #580 created.",
};

interface Setup {
  readonly read?: ReadForm;
  readonly create?: CreateCard;
}

/** The dialog open on `scope` inside a router (its connect link is a router link). */
async function mount({ read = async () => form, create = async () => created }: Setup = {}) {
  const sent: CreateFizzyCard[] = [];
  const done: CreatedFizzyCard[] = [];
  let closes = 0;

  const recording: CreateCard = (source, body) => {
    sent.push(body);

    return create(source, body);
  };

  const rootRoute = createRootRoute({
    component: () => (
      <FizzyCardDialog
        scope={scope}
        open
        onClose={() => {
          closes += 1;
        }}
        onCreated={(card) => done.push(card)}
        read={read}
        create={recording}
      />
    ),
  });

  const settingsRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/settings/integrations",
  });

  const router = createRouter({
    routeTree: rootRoute.addChildren([settingsRoute]),
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return { sent, done, closes: () => closes };
}

function dialog() {
  return screen.getByRole("dialog", { name: "Create Fizzy card" });
}

/** A form control's current value, for the input, select and textarea the dialog draws. */
function fieldValue(element: HTMLElement): string {
  if (
    element instanceof HTMLInputElement ||
    element instanceof HTMLSelectElement ||
    element instanceof HTMLTextAreaElement
  ) {
    return element.value;
  }

  throw new Error(`${element.tagName} isn't a form field`);
}

describe("Create Fizzy card", () => {
  it("quotes the source and prefills the form, with the board focused", async () => {
    await mount();

    const board = await screen.findByLabelText("Board");

    await waitFor(() => expect(document.activeElement).toBe(board));
    expect(within(dialog()).getByText("general")).toBeTruthy();
    expect(dialog().textContent).toContain("Create Fizzy card in general");
    expect(within(dialog()).getByText("— Maya")).toBeTruthy();
    expect(fieldValue(board)).toBe("");
    expect(
      within(board)
        .getAllByRole("option")
        .map((option) => option.textContent),
    ).toEqual(["Choose a board", "Engineering", "Support"]);
    expect(fieldValue(screen.getByLabelText("Title"))).toBe(form.title);
    expect(screen.getByLabelText("Title").getAttribute("maxlength")).toBe("500");
    expect(fieldValue(screen.getByLabelText("Description"))).toBe(form.description);
    expect(screen.getByLabelText("Description").getAttribute("maxlength")).toBe("50000");
    expect(
      screen.getByText(
        "Creates the card as Riel in Smart Data, then posts a reply here with the new card.",
      ),
    ).toBeTruthy();
  });

  it("asks for a board before sending anything", async () => {
    const user = userEvent.setup();
    const { sent } = await mount();

    await user.click(await screen.findByRole("button", { name: "Create card" }));

    expect(sent).toEqual([]);
    expect(screen.getByText("Choose a board and enter a title.")).toBeTruthy();
    expect(screen.getByText("Choose a board.")).toBeTruthy();
    expect(document.activeElement).toBe(screen.getByLabelText("Board"));
  });

  it("creates the card once and hands the result over", async () => {
    const user = userEvent.setup();
    const { sent, done } = await mount();

    await user.selectOptions(await screen.findByLabelText("Board"), "support");
    await user.click(screen.getByRole("button", { name: "Create card" }));

    await waitFor(() => expect(done).toEqual([created]));
    expect(sent).toEqual([
      { boardId: "support", title: form.title, description: form.description },
    ]);
  });

  it("puts the server's validation errors on their fields", async () => {
    const user = userEvent.setup();

    await mount({
      create: async () => {
        throw new ActionError("Validation", "Choose a board and enter a title.", {
          title: ["Enter a title."],
        });
      },
    });

    await user.selectOptions(await screen.findByLabelText("Board"), "support");
    await user.click(screen.getByRole("button", { name: "Create card" }));

    expect(await screen.findByText("Enter a title.")).toBeTruthy();
    expect(screen.getByText("Choose a board and enter a title.")).toBeTruthy();
    expect(document.activeElement).toBe(screen.getByLabelText("Title"));
  });

  it("keeps the form with Fizzy's refusal, and doesn't retry", async () => {
    const user = userEvent.setup();
    const message = "That Fizzy token is read-only. Generate a Read + Write token to create cards.";

    const { sent } = await mount({
      create: async () => {
        throw new ActionError("FizzyReadOnly", message);
      },
    });

    await user.selectOptions(await screen.findByLabelText("Board"), "engineering");
    await user.click(screen.getByRole("button", { name: "Create card" }));

    expect((await screen.findByRole("alert")).textContent).toContain(message);
    expect(sent).toHaveLength(1);
    expect(screen.getByLabelText("Title")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Create card" })).toBeTruthy();
  });

  it("switches to the way to connect when the connection is gone", async () => {
    const user = userEvent.setup();

    await mount({
      create: async () => {
        throw new ActionError("FizzyNotConnected", "Connect Fizzy on your profile first.");
      },
    });

    await user.selectOptions(await screen.findByLabelText("Board"), "engineering");
    await user.click(screen.getByRole("button", { name: "Create card" }));

    expect(await screen.findByText("Connect Fizzy on your profile first.")).toBeTruthy();
    expect(
      screen.getByText(
        "Connect your Fizzy account first: card previews and creation use your own Fizzy access.",
      ),
    ).toBeTruthy();

    const connect = screen.getByRole("link", { name: "Connect Fizzy on your profile" });

    expect(connect.getAttribute("href")).toBe("/settings/integrations#integration-fizzy");
    await waitFor(() => expect(document.activeElement).toBe(connect));
    expect(screen.queryByLabelText("Board")).toBeNull();
    expect(screen.queryByRole("button", { name: "Create card" })).toBeNull();
  });

  it("shows the source and the way to connect for a disconnected viewer", async () => {
    await mount({ read: async () => ({ ...form, connected: false, boards: [] }) });

    expect(await screen.findByText("— Maya")).toBeTruthy();
    expect(
      screen.getByText(
        "Connect your Fizzy account first: card previews and creation use your own Fizzy access.",
      ),
    ).toBeTruthy();
    expect(screen.queryByLabelText("Board")).toBeNull();
  });

  it("links the card a failed reply left behind, and only offers to close", async () => {
    const user = userEvent.setup();

    const message =
      "Fizzy card #580 created, but the reply could not be posted (Body is too long).";

    const { sent, closes } = await mount({
      create: async () => {
        throw new ActionError(
          "FizzyReplyFailed",
          message,
          {},
          {
            number: "580",
            url: "https://fizzy.test/cards/580",
          },
        );
      },
    });

    await user.selectOptions(await screen.findByLabelText("Board"), "engineering");
    await user.click(screen.getByRole("button", { name: "Create card" }));

    expect(await screen.findByText(message)).toBeTruthy();
    expect(screen.getByRole("link", { name: /Open Fizzy card #580/ }).getAttribute("href")).toBe(
      "https://fizzy.test/cards/580",
    );
    expect(screen.queryByRole("button", { name: "Create card" })).toBeNull();

    const close = within(dialog()).getAllByRole("button", { name: "Close" }).at(-1);

    if (close === undefined) throw new Error("the dialog has no Close button");

    await waitFor(() => expect(document.activeElement).toBe(close));
    await user.click(close);
    expect(closes()).toBe(1);
    expect(sent).toHaveLength(1);
  });

  it("says when the form can't load, and reads it again on request", async () => {
    const user = userEvent.setup();
    let reads = 0;

    await mount({
      read: async () => {
        reads += 1;

        if (reads === 1)
          throw new ActionError("FizzyUnreachable", "Could not reach Fizzy. Try again.");

        return form;
      },
    });

    expect(
      await screen.findByText("Couldn't load the form: Could not reach Fizzy. Try again."),
    ).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Try again" }));
    expect(await screen.findByLabelText("Board")).toBeTruthy();
  });

  it("waits for a create in flight before Escape closes it", async () => {
    const user = userEvent.setup();
    const pending = Promise.withResolvers<CreatedFizzyCard>();
    const { closes, done } = await mount({ create: () => pending.promise });

    await user.selectOptions(await screen.findByLabelText("Board"), "engineering");
    await user.click(screen.getByRole("button", { name: "Create card" }));
    await user.keyboard("{Escape}");
    expect(closes()).toBe(0);

    await act(async () => pending.resolve(created));
    expect(done).toEqual([created]);

    await user.keyboard("{Escape}");
    expect(closes()).toBe(1);
  });
});
