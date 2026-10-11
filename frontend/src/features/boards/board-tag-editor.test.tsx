import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { BoardTagCatalog } from "../../gen/BoardTagCatalog.ts";
import { initialState } from "../../store/state.ts";
import { mutations, store } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { BoardTagEditor } from "./board-tag-editor.tsx";

const ROOM = 900;

afterEach(() => {
  vi.restoreAllMocks();
  store.setState(initialState, true);
});

/** A server that keeps one catalog and answers every write with it, as the real one does. */
function installCatalog(start: Omit<BoardTagCatalog, "roomId">) {
  let catalog: BoardTagCatalog = { roomId: ROOM, ...start };

  const answer = async (next: BoardTagCatalog = catalog) => {
    catalog = next;
    mutations.setBoardTagCatalog(catalog);

    return catalog;
  };

  vi.spyOn(actions.boards, "tagCatalog").mockImplementation(() => answer());

  return {
    reorder: vi.spyOn(actions.boards, "reorderTags").mockImplementation((_, ids) =>
      answer({
        ...catalog,
        tags: ids.flatMap((id, position) => {
          const tag = catalog.tags.find((candidate) => candidate.id === id);

          return tag === undefined ? [] : [{ ...tag, position }];
        }),
      }),
    ),
    create: vi.spyOn(actions.boards, "createTag").mockImplementation((_, body) =>
      answer({
        ...catalog,
        tags: [
          ...catalog.tags,
          { id: 50, name: body.name, emoji: body.emoji, position: catalog.tags.length },
        ],
      }),
    ),
    remove: vi.spyOn(actions.boards, "deleteTag").mockImplementation((_, id) =>
      answer({
        ...catalog,
        tags: catalog.tags.filter((tag) => tag.id !== id),
        defaultBoardTagId: catalog.defaultBoardTagId === id ? null : catalog.defaultBoardTagId,
      }),
    ),
    policy: vi
      .spyOn(actions.boards, "updateTagPolicy")
      .mockImplementation((_, body) => answer({ ...catalog, ...body })),
  };
}

const bug = { id: 1, name: "Bug", emoji: "🐞", position: 0 };

const idea = { id: 2, name: "Idea", emoji: null, position: 1 };

describe("the board tag editor", () => {
  it("moves a tag with the keyboard and saves the whole order", async () => {
    const server = installCatalog({ tags: [bug, idea], tagsRequired: false, defaultBoardTagId: 2 });

    render(<BoardTagEditor roomId={ROOM} />);

    const handle = await screen.findByRole("button", { name: "Move Bug, position 1 of 2" });

    expect(screen.getByText("Default")).toBeTruthy();

    fireEvent.keyDown(handle, { key: "ArrowDown" });

    await waitFor(() => expect(server.reorder).toHaveBeenCalledWith(ROOM, [2, 1]));
    expect(await screen.findByRole("button", { name: "Move Bug, position 2 of 2" })).toBeTruthy();
    expect(screen.getByText("Bug moved to position 2 of 2.")).toBeTruthy();

    fireEvent.keyDown(screen.getByRole("button", { name: "Move Bug, position 2 of 2" }), {
      key: "End",
    });
    expect(server.reorder).toHaveBeenCalledTimes(1);
  });

  it("adds a tag on Enter and checks the name before asking", async () => {
    const server = installCatalog({ tags: [bug], tagsRequired: false, defaultBoardTagId: null });
    const user = userEvent.setup();

    render(<BoardTagEditor roomId={ROOM} />);

    const name = await screen.findByLabelText("New tag");

    await user.type(name, "bug{Enter}");

    expect(await screen.findByText("“Bug” is already a tag here.")).toBeTruthy();
    expect(server.create).not.toHaveBeenCalled();

    await user.clear(name);
    await user.type(name, "  Shipped {Enter}");

    await waitFor(() =>
      expect(server.create).toHaveBeenCalledWith(ROOM, { name: "Shipped", emoji: null }),
    );
    expect(
      await screen.findByRole("button", { name: "Move Shipped, position 2 of 2" }),
    ).toBeTruthy();
    expect(screen.getByText("2 of 20")).toBeTruthy();
  });

  it("says what a delete does and lifts a requirement the last tag held up", async () => {
    const server = installCatalog({ tags: [bug], tagsRequired: true, defaultBoardTagId: 1 });
    const user = userEvent.setup();

    render(<BoardTagEditor roomId={ROOM} />);

    await user.click(await screen.findByRole("button", { name: "Bug options" }));
    await user.click(await screen.findByRole("menuitem", { name: /Delete/ }));

    const confirm = await screen.findByRole("alertdialog", { name: "Delete “Bug”?" });

    expect(within(confirm).getByText("Every post that has it loses it.")).toBeTruthy();
    expect(within(confirm).getByText("New posts stop starting with it.")).toBeTruthy();
    expect(
      within(confirm).getByText("It's the last tag, so new posts stop requiring one."),
    ).toBeTruthy();

    await user.click(within(confirm).getByRole("button", { name: "Delete tag" }));

    await waitFor(() => expect(server.remove).toHaveBeenCalledWith(ROOM, 1));
    await waitFor(() =>
      expect(server.policy).toHaveBeenCalledWith(ROOM, {
        tagsRequired: false,
        defaultBoardTagId: null,
      }),
    );
    expect(await screen.findByText("No tags yet. Add the first below.")).toBeTruthy();
  });

  it("sets the default and the requirement from the rules", async () => {
    const server = installCatalog({
      tags: [bug, idea],
      tagsRequired: false,
      defaultBoardTagId: null,
    });

    const user = userEvent.setup();

    render(<BoardTagEditor roomId={ROOM} />);

    await user.selectOptions(await screen.findByLabelText("Start new posts with"), "2");
    await waitFor(() =>
      expect(server.policy).toHaveBeenLastCalledWith(ROOM, {
        tagsRequired: false,
        defaultBoardTagId: 2,
      }),
    );

    await user.click(screen.getByRole("switch", { name: /Require a tag on new posts/ }));
    await waitFor(() =>
      expect(server.policy).toHaveBeenLastCalledWith(ROOM, {
        tagsRequired: true,
        defaultBoardTagId: 2,
      }),
    );
  });
});
