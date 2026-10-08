import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { userFixture } from "../../api/testing.ts";
import type { ThreadDetail } from "../../gen/ThreadDetail.ts";
import type { WorkHandoffReceiver } from "../../gen/WorkHandoffReceiver.ts";
import { initialState } from "../../store/state.ts";
import { mutations, store } from "../../store/store.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { boardDetail, boardThread } from "../../test/board-fixtures.ts";
import { toastSnapshot } from "../../ui/toast-store.ts";
import { HandoffDialog } from "./handoff-dialog.tsx";

const THREAD = 9004;

const EMBER = { agentId: 41, userId: 30 } as const satisfies WorkHandoffReceiver;

const ORBIT = { agentId: 42, userId: 31 } as const satisfies WorkHandoffReceiver;

function detail(
  receivers: readonly WorkHandoffReceiver[],
  change: { readonly manage?: boolean; readonly tracked?: boolean } = {},
): ThreadDetail {
  const base = boardDetail({ ...boardThread(THREAD), name: "Cursor pagination" });
  const thread = change.tracked === false ? { ...base.thread, work: null } : base.thread;

  return {
    ...base,
    thread,
    permissions: { ...base.permissions, canManageWork: change.manage ?? true },
    work:
      change.tracked === false || base.work === null
        ? null
        : { ...base.work, handoffReceivers: [...receivers] },
    users: [
      ...base.users,
      { ...userFixture(30, "Ember"), role: "bot" },
      { ...userFixture(31, "Orbit"), role: "bot" },
    ],
  };
}

function setup(
  receivers: readonly WorkHandoffReceiver[] = [EMBER, ORBIT],
  change: { readonly manage?: boolean; readonly tracked?: boolean } = {},
) {
  mutations.loadThreadDetail(detail(receivers, change), 0);
  const onClose = vi.fn();
  const user = userEvent.setup();

  render(<HandoffDialog threadId={THREAD} open onClose={onClose} />);

  return { onClose, user };
}

function toasts(): string[] {
  return toastSnapshot().map((each) => each.title);
}

afterEach(() => {
  vi.restoreAllMocks();
  store.setState(initialState, true);
});

describe("the handoff dialog", () => {
  it("names the work and lists the agents it can go to", () => {
    setup();

    expect(screen.getByRole("heading", { name: "Hand off “Cursor pagination”" })).toBeTruthy();

    const select = screen.getByLabelText("Receiving agent");

    expect([...select.querySelectorAll("option")].map((option) => option.textContent)).toEqual([
      "Choose an agent",
      "Ember",
      "Orbit",
    ]);
  });

  it("checks the receiver, summary, links and questions before sending", async () => {
    const handoff = vi.spyOn(actions.work, "handoff");
    const { user } = setup();

    await user.click(screen.getByRole("button", { name: "Hand off" }));

    expect(screen.getByText("Choose the agent to hand this work to.")).toBeTruthy();
    expect(screen.getByText("Summary can't be blank.")).toBeTruthy();

    await user.selectOptions(screen.getByLabelText("Receiving agent"), "Orbit");
    await user.type(screen.getByLabelText("Summary"), "Ready for the endpoint");
    await user.type(screen.getByLabelText("Links"), "ftp://example.com/spec");
    await user.type(
      screen.getByLabelText("Open questions"),
      Array.from({ length: 11 }, (_, index) => `Question ${index}?`).join("\n"),
    );
    await user.click(screen.getByRole("button", { name: "Hand off" }));

    expect(screen.queryByText("Choose the agent to hand this work to.")).toBeNull();
    expect(screen.getByText("Links must be http(s) URLs.")).toBeTruthy();
    expect(screen.getByText("Open questions are limited to 10 per handoff.")).toBeTruthy();
    expect(handoff).not.toHaveBeenCalled();
  });

  it("hands the work off with one entry per line, disabled while it goes, then says to whom", async () => {
    const pending = Promise.withResolvers<void>();
    const handoff = vi.spyOn(actions.work, "handoff").mockReturnValue(pending.promise);
    const { onClose, user } = setup();

    await user.selectOptions(screen.getByLabelText("Receiving agent"), "Ember");
    await user.type(screen.getByLabelText("Summary"), "Cursor shape agreed");
    await user.type(
      screen.getByLabelText("Links"),
      "https://example.com/spec\n\n https://example.com/spec \nhttps://example.com/pr/4",
    );
    await user.type(screen.getByLabelText("Open questions"), "Page size?");
    await user.click(screen.getByRole("button", { name: "Hand off" }));

    expect(handoff).toHaveBeenCalledWith(THREAD, {
      receiverAgentId: EMBER.agentId,
      summary: "Cursor shape agreed",
      links: ["https://example.com/spec", "https://example.com/pr/4"],
      openQuestions: ["Page size?"],
    });

    for (const label of ["Receiving agent", "Summary", "Links", "Open questions"]) {
      expect(screen.getByLabelText(label)).toHaveProperty("disabled", true);
    }

    expect(screen.getByRole("button", { name: /Hand off/ }).getAttribute("aria-busy")).toBe("true");

    // A second submit while the first is out sends nothing more.
    await user.keyboard("{Control>}{Enter}{/Control}");
    expect(handoff).toHaveBeenCalledTimes(1);
    expect(onClose).not.toHaveBeenCalled();

    await act(async () => pending.resolve());

    expect(toasts()).toContain("Work handed off to Ember.");
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("shows the server's field errors under their fields and lets them try again", async () => {
    vi.spyOn(actions.work, "handoff").mockRejectedValue(
      new ActionError("Validation", "Summary is too long (maximum is 2000 characters)", {
        summary: ["is too long (maximum is 2000 characters)"],
        receiverAgentId: ["Receiver must hold the manage_threads capability in this room"],
      }),
    );

    const { onClose, user } = setup();

    await user.selectOptions(screen.getByLabelText("Receiving agent"), "Orbit");
    await user.type(screen.getByLabelText("Summary"), "Over to you");
    await user.click(screen.getByRole("button", { name: "Hand off" }));

    expect(
      await screen.findByText("Summary is too long (maximum is 2000 characters)."),
    ).toBeTruthy();
    expect(
      screen.getByText("Receiver must hold the manage_threads capability in this room."),
    ).toBeTruthy();
    expect(screen.getByLabelText("Summary")).toHaveProperty("disabled", false);
    expect(onClose).not.toHaveBeenCalled();

    // The first field in trouble takes focus.
    await waitFor(() =>
      expect(document.activeElement).toBe(screen.getByLabelText("Receiving agent")),
    );

    // Editing the field clears its error.
    await user.type(screen.getByLabelText("Summary"), "!");
    expect(screen.queryByText("Summary is too long (maximum is 2000 characters).")).toBeNull();
  });

  it("shows a refusal that names no field as the form's alert, in the server's words", async () => {
    vi.spyOn(actions.work, "handoff")
      .mockRejectedValueOnce(
        new ActionError("Validation", "This thread isn't tracked as work", {
          base: ["This thread isn't tracked as work"],
        }),
      )
      .mockRejectedValueOnce(new ActionError("Forbidden", "You cannot manage work in this thread"));

    const { user } = setup();

    await user.selectOptions(screen.getByLabelText("Receiving agent"), "Orbit");
    await user.type(screen.getByLabelText("Summary"), "Over to you");
    await user.click(screen.getByRole("button", { name: "Hand off" }));
    expect((await screen.findByRole("alert")).textContent).toBe(
      "This thread isn't tracked as work",
    );

    await user.click(screen.getByRole("button", { name: "Hand off" }));
    expect((await screen.findByRole("alert")).textContent).toBe(
      "You cannot manage work in this thread",
    );
  });

  it("chooses a lone receiver", () => {
    setup([EMBER]);

    expect(screen.getByLabelText("Receiving agent")).toHaveProperty("value", String(EMBER.agentId));
  });

  it("says when no agent in the room can take the work", () => {
    setup([]);

    expect(screen.getByText(/No agent here can take this work/)).toBeTruthy();
    expect(screen.getByLabelText("Receiving agent")).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: "Hand off" })).toHaveProperty("disabled", true);
  });

  it("sends someone who can't manage the work back to the thread, saying why", () => {
    const { onClose } = setup([EMBER], { manage: false });

    expect(onClose).toHaveBeenCalledTimes(1);
    expect(toasts()).toContain("You cannot manage work in this thread");
  });

  it("sends an untracked thread back too", () => {
    const { onClose } = setup([], { tracked: false });

    expect(onClose).toHaveBeenCalledTimes(1);
    expect(toasts()).toContain("This thread isn't tracked as work");
  });
});
