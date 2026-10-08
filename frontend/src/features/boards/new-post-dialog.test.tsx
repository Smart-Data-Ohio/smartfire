import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { initialState } from "../../store/state.ts";
import { store } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { boardDetail } from "../../test/board-fixtures.ts";
import { NewPostDialog } from "./new-post-dialog.tsx";

afterEach(() => {
  vi.restoreAllMocks();
  store.setState(initialState, true);
});

describe("the new-post dialog", () => {
  it("doesn't open a post whose creation finishes after the room changed", async () => {
    let finish: (detail: ReturnType<typeof boardDetail>) => void = () => undefined;

    vi.spyOn(actions.boards, "postForm").mockResolvedValue({
      ownerCandidates: [],
      tagSuggestions: [],
      users: [],
    });
    vi.spyOn(actions.boards, "createPost").mockImplementation(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );

    const created = vi.fn();
    const user = userEvent.setup();

    const dialog = (roomId: number) => (
      // The room pane is keyed by room, so a room change mounts a new dialog.
      <NewPostDialog
        key={roomId}
        roomId={roomId}
        open
        onClose={() => undefined}
        onCreated={created}
      />
    );

    const { rerender } = render(dialog(900));

    await user.type(screen.getByLabelText("Title"), "Fix the import");
    await user.click(screen.getByRole("button", { name: "Create post" }));
    expect(actions.boards.createPost).toHaveBeenCalledTimes(1);

    rerender(dialog(901));

    await act(async () => {
      finish(boardDetail());
    });

    expect(created).not.toHaveBeenCalled();
  });
});
