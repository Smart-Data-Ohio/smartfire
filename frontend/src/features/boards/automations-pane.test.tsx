import { act, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { BoardAutomations } from "../../gen/BoardAutomations.ts";
import { initialState } from "../../store/state.ts";
import { mutations, store } from "../../store/store.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { BOARD, boardAutomations } from "../../test/board-fixtures.ts";
import { BoardAutomationsPane } from "./automations-pane.tsx";

afterEach(() => {
  vi.restoreAllMocks();
  store.setState(initialState, true);
});

function deferred<T>() {
  let resolve: (value: T) => void = () => undefined;

  const promise = new Promise<T>((done) => {
    resolve = done;
  });

  return { promise, resolve };
}

// SAFETY: every label these tests look up names an <input> (or the <select>, read only for
// `disabled` and `value`, which both elements have).
const field = (label: string) => screen.getByLabelText(label) as HTMLInputElement;

// SAFETY: the "button" role is only ever a <button> in this pane.
const button = (name: string) => screen.getByRole("button", { name }) as HTMLButtonElement;

/** Someone else changed the board's automations, as the sync engine reports it. */
function signal(seq: number) {
  act(() =>
    mutations.applyEvents(
      [
        {
          type: "board.automations.changed",
          seq,
          topic: `room:${BOARD}`,
          data: { roomId: BOARD },
        },
      ],
      Date.now(),
    ),
  );
}

const withTimers = (slaTimers: BoardAutomations["slaTimers"]): BoardAutomations => ({
  ...boardAutomations(),
  slaTimers,
});

describe("the board automations pane", () => {
  it("runs one change at a time, with every control disabled until it answers", async () => {
    vi.spyOn(actions.boards, "automations").mockResolvedValue(boardAutomations());

    const removal = deferred<BoardAutomations>();

    vi.spyOn(actions.boards, "removeTagRule").mockReturnValue(removal.promise);

    const add = vi.spyOn(actions.boards, "addTagRule");
    const save = vi.spyOn(actions.boards, "saveSlaTimers");
    const user = userEvent.setup();

    render(<BoardAutomationsPane roomId={BOARD} />);
    await user.click(
      await screen.findByRole("button", { name: "Remove auto-assign rule for bug" }),
    );

    expect(field("Tag").disabled).toBe(true);
    expect(button("Add rule").disabled).toBe(true);
    expect(field("Planned nudge minutes").disabled).toBe(true);
    expect(button("Remove auto-assign rule for bug").disabled).toBe(true);

    // Enter in a field submits even with the button disabled; neither form starts a change.
    fireEvent.submit(screen.getByRole("form", { name: "Add an auto-assign rule" }));
    fireEvent.submit(screen.getByRole("form", { name: "SLA timers" }));
    expect(add).not.toHaveBeenCalled();
    expect(save).not.toHaveBeenCalled();

    await act(async () => removal.resolve({ ...boardAutomations(), tagRules: [] }));

    expect(screen.queryByRole("button", { name: "Remove auto-assign rule for bug" })).toBeNull();
    expect(field("Tag").disabled).toBe(false);
    expect(field("Planned nudge minutes").disabled).toBe(false);
  });

  it("keeps a slow refetch from undoing the change that answered after it was asked", async () => {
    const refetch = deferred<BoardAutomations>();

    vi.spyOn(actions.boards, "automations")
      .mockResolvedValueOnce(boardAutomations())
      .mockReturnValueOnce(refetch.promise);
    vi.spyOn(actions.boards, "removeTagRule").mockResolvedValue({
      ...boardAutomations(),
      tagRules: [],
    });

    const user = userEvent.setup();

    render(<BoardAutomationsPane roomId={BOARD} />);
    await screen.findByRole("button", { name: "Remove auto-assign rule for bug" });
    signal(1);
    expect(actions.boards.automations).toHaveBeenCalledTimes(2);

    await user.click(button("Remove auto-assign rule for bug"));
    expect(screen.queryByRole("button", { name: "Remove auto-assign rule for bug" })).toBeNull();

    // The refetch was asked before the removal and still holds the rule.
    await act(async () => refetch.resolve(boardAutomations()));

    expect(screen.queryByRole("button", { name: "Remove auto-assign rule for bug" })).toBeNull();
  });

  it("refetches when another client changes the automations, once its own change is done", async () => {
    const removal = deferred<BoardAutomations>();

    const theirs: BoardAutomations = {
      ...boardAutomations(),
      tagRules: [{ id: 2, tag: "ops", assigneeId: 7 }],
    };

    vi.spyOn(actions.boards, "automations")
      .mockResolvedValueOnce(boardAutomations())
      .mockResolvedValueOnce(theirs);
    vi.spyOn(actions.boards, "removeTagRule").mockReturnValue(removal.promise);

    const user = userEvent.setup();

    render(<BoardAutomationsPane roomId={BOARD} />);
    await user.click(
      await screen.findByRole("button", { name: "Remove auto-assign rule for bug" }),
    );
    signal(1);
    expect(actions.boards.automations).toHaveBeenCalledTimes(1);

    await act(async () => removal.resolve({ ...boardAutomations(), tagRules: [] }));

    expect(actions.boards.automations).toHaveBeenCalledTimes(2);
    expect(
      await screen.findByRole("button", { name: "Remove auto-assign rule for ops" }),
    ).toBeDefined();
  });

  it("fills untouched timer fields from fresh settings and saves only the rows changed", async () => {
    const fresh = withTimers([
      { status: "planned", nudgeAfterMinutes: 60, escalateAfterMinutes: 120 },
      { status: "blocked", nudgeAfterMinutes: 5, escalateAfterMinutes: 10 },
    ]);

    vi.spyOn(actions.boards, "automations")
      .mockResolvedValueOnce(boardAutomations())
      .mockResolvedValueOnce(fresh);

    const save = vi.spyOn(actions.boards, "saveSlaTimers").mockResolvedValue(fresh);
    const user = userEvent.setup();

    render(<BoardAutomationsPane roomId={BOARD} />);
    expect((await screen.findByLabelText("Planned nudge minutes")).getAttribute("value")).toBe(
      "1440",
    );

    await user.type(field("In progress nudge minutes"), "30");
    await user.type(field("In progress escalation minutes"), "90");
    await user.clear(field("Planned escalation minutes"));
    await user.type(field("Planned escalation minutes"), "3000");
    signal(1);

    await vi.waitFor(() => expect(field("Blocked nudge minutes").value).toBe("5"));
    expect(field("Planned nudge minutes").value).toBe("60");
    expect(field("Planned escalation minutes").value).toBe("3000");
    expect(field("In progress nudge minutes").value).toBe("30");
    expect(field("In progress escalation minutes").value).toBe("90");

    await user.click(button("Save SLA timers"));

    expect(save).toHaveBeenCalledWith(BOARD, {
      planned: { nudgeAfterMinutes: 60, escalateAfterMinutes: 3000 },
      inProgress: { nudgeAfterMinutes: 30, escalateAfterMinutes: 90 },
    });
  });

  it("refetches when its room resyncs, keeping the timer fields being edited", async () => {
    const fresh = withTimers([
      { status: "planned", nudgeAfterMinutes: 60, escalateAfterMinutes: 120 },
      { status: "blocked", nudgeAfterMinutes: 5, escalateAfterMinutes: 10 },
    ]);

    const load = vi
      .spyOn(actions.boards, "automations")
      .mockResolvedValueOnce(boardAutomations())
      .mockResolvedValueOnce(fresh);

    const user = userEvent.setup();

    render(<BoardAutomationsPane roomId={BOARD} />);
    await screen.findByLabelText("Planned nudge minutes");
    await user.clear(field("Planned escalation minutes"));
    await user.type(field("Planned escalation minutes"), "3000");
    await user.type(field("In progress nudge minutes"), "30");

    // As the sync engine does when the board's room resyncs.
    act(() => mutations.boardAutomationsChanged(BOARD));

    await vi.waitFor(() => expect(field("Blocked nudge minutes").value).toBe("5"));
    expect(load).toHaveBeenCalledTimes(2);
    expect(field("Planned nudge minutes").value).toBe("60");
    expect(field("Planned escalation minutes").value).toBe("3000");
    expect(field("In progress nudge minutes").value).toBe("30");
  });

  it("locks the fields while a change saves, so nothing typed meanwhile is dropped", async () => {
    vi.spyOn(actions.boards, "automations").mockResolvedValue(boardAutomations());

    const saving = deferred<BoardAutomations>();
    const adding = deferred<BoardAutomations>();

    vi.spyOn(actions.boards, "saveSlaTimers").mockReturnValue(saving.promise);
    vi.spyOn(actions.boards, "addTagRule").mockReturnValue(adding.promise);

    const user = userEvent.setup();

    render(<BoardAutomationsPane roomId={BOARD} />);
    await user.clear(await screen.findByLabelText("Planned nudge minutes"));
    await user.type(field("Planned nudge minutes"), "30");
    await user.click(button("Save SLA timers"));

    for (const label of [
      "Planned nudge minutes",
      "Planned escalation minutes",
      "Blocked nudge minutes",
      "Tag",
    ]) {
      expect(field(label).disabled, label).toBe(true);
    }

    await act(async () =>
      saving.resolve(
        withTimers([{ status: "planned", nudgeAfterMinutes: 30, escalateAfterMinutes: 2880 }]),
      ),
    );
    expect(field("Planned nudge minutes").value).toBe("30");

    await user.type(field("Tag"), "ops");
    await user.selectOptions(screen.getByLabelText("Assign to"), "7");
    await user.click(button("Add rule"));
    expect(field("Tag").disabled).toBe(true);
    expect(field("Assign to").disabled).toBe(true);

    await act(async () =>
      adding.resolve({
        ...boardAutomations(),
        tagRules: [...boardAutomations().tagRules, { id: 2, tag: "ops", assigneeId: 7 }],
      }),
    );
    expect(field("Tag").value).toBe("");
    expect(field("Tag").disabled).toBe(false);
  });

  it("explains a member who cannot administer, without offering a retry", async () => {
    vi.spyOn(actions.boards, "automations").mockRejectedValue(
      new ActionError("Forbidden", "Not allowed"),
    );

    render(<BoardAutomationsPane roomId={BOARD} />);

    expect(await screen.findByText("Automations are limited")).toBeTruthy();
    expect(
      screen.getByText("Only the person who made this board and administrators can open them."),
    ).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Try again" })).toBeNull();
  });

  it("explains a board that isn't available, instead of a blank pane", async () => {
    vi.spyOn(actions.boards, "automations").mockRejectedValue(
      new ActionError("NotFound", "Not found"),
    );

    render(<BoardAutomationsPane roomId={BOARD} />);

    expect(await screen.findByText("This board isn't available")).toBeTruthy();
    expect(screen.getByText("You aren't in it, or it isn't a board.")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Try again" })).toBeNull();
  });
});
