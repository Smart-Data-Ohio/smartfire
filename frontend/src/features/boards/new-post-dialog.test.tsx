import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { initialState } from "../../store/state.ts";
import { mutations, store } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { boardDetail } from "../../test/board-fixtures.ts";
import { toastSnapshot } from "../../ui/toast-store.ts";
import { NewPostDialog } from "./new-post-dialog.tsx";

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  store.setState(initialState, true);
});

function installUploads(pending = false) {
  vi.spyOn(actions.boards, "postForm").mockResolvedValue({
    ownerCandidates: [],
    tagSuggestions: [],
    users: [],
    tags: [],
    tagsRequired: false,
    defaultBoardTagId: null,
  });
  vi.spyOn(actions.messages, "startUpload").mockImplementation(async (body) => ({
    signedId: `signed-${body.filename}`,
    uploadUrl: "/put",
  }));
  vi.stubGlobal("matchMedia", (query: string) =>
    Object.assign(new EventTarget(), { matches: true, media: query, onchange: null }),
  );

  const aborted = vi.fn();

  vi.stubGlobal(
    "XMLHttpRequest",
    class {
      status = 204;
      upload = { onprogress: null };
      onload: (() => void) | null = null;
      onerror = null;
      onabort: (() => void) | null = null;
      open() {}
      setRequestHeader() {}
      abort() {
        aborted();
        this.onabort?.();
      }

      send() {
        if (!pending) queueMicrotask(() => this.onload?.());
      }
    },
  );

  return aborted;
}

describe("the new-post dialog", () => {
  it("cancels pending uploads when navigation closes the dialog, and reopens with an empty tray", async () => {
    const aborted = installUploads(true);

    const dialog = (open: boolean) => (
      <NewPostDialog roomId={900} open={open} onClose={() => {}} onCreated={() => {}} />
    );

    const { container, rerender } = render(dialog(true));
    const picker = container.querySelector<HTMLInputElement>('input[type="file"]');

    if (picker === null) throw new Error("no file input");
    fireEvent.change(picker, { target: { files: [new File(["one"], "pending.txt")] } });
    await waitFor(() => expect(container.querySelector('[data-phase="uploading"]')).not.toBeNull());
    rerender(dialog(false));
    await waitFor(() => expect(aborted).toHaveBeenCalledOnce());
    rerender(dialog(true));
    expect(screen.queryByRole("button", { name: "Remove pending.txt" })).toBeNull();
  });

  it("ignores files pasted during post creation so none remain hidden after success", async () => {
    installUploads();

    const pending = Promise.withResolvers<ReturnType<typeof boardDetail>>();

    vi.spyOn(actions.boards, "createPost").mockReturnValue(pending.promise);

    const { container } = render(
      <NewPostDialog roomId={900} open onClose={() => {}} onCreated={() => {}} />,
    );

    fireEvent.change(screen.getByLabelText("Title"), { target: { value: "The upload" } });

    const picker = container.querySelector<HTMLInputElement>('input[type="file"]');

    if (picker === null) throw new Error("no file input");
    fireEvent.change(picker, { target: { files: [new File(["one"], "one.txt")] } });
    await waitFor(() => expect(container.querySelector('[data-phase="done"]')).not.toBeNull());
    fireEvent.click(screen.getByRole("button", { name: "Create post" }));
    await waitFor(() => expect(actions.boards.createPost).toHaveBeenCalledOnce());
    fireEvent.paste(screen.getByLabelText(/Brief/), {
      clipboardData: { files: [new File(["later"], "later.txt")] },
    });
    expect(actions.messages.startUpload).toHaveBeenCalledTimes(1);

    await act(async () => pending.resolve(boardDetail()));

    expect(container.querySelectorAll(".tray-chip")).toHaveLength(0);
  });

  it("creates a file-only brief with the shared grouped attachment tray", async () => {
    vi.spyOn(actions.boards, "postForm").mockResolvedValue({
      ownerCandidates: [],
      tagSuggestions: [],
      users: [],
      tags: [],
      tagsRequired: false,
      defaultBoardTagId: null,
    });

    const create = vi
      .spyOn(actions.boards, "createPost")
      .mockRejectedValue(new Error("Keep draft"));

    vi.spyOn(actions.messages, "startUpload").mockImplementation(async (body) => ({
      signedId: `signed-${body.filename}`,
      uploadUrl: "/put",
    }));
    vi.stubGlobal("matchMedia", (query: string) =>
      Object.assign(new EventTarget(), { matches: true, media: query, onchange: null }),
    );
    vi.stubGlobal(
      "XMLHttpRequest",
      class {
        status = 204;
        upload = { onprogress: null };
        onload: (() => void) | null = null;
        onerror = null;
        onabort = null;
        open() {}
        setRequestHeader() {}
        abort() {}
        send() {
          queueMicrotask(() => this.onload?.());
        }
      },
    );

    const { container } = render(
      <NewPostDialog roomId={900} open onClose={() => undefined} onCreated={() => undefined} />,
    );

    const picker = container.querySelector<HTMLInputElement>('input[type="file"]');

    expect(picker).not.toBeNull();

    if (picker === null) throw new Error("no file input");
    fireEvent.change(screen.getByLabelText("Title"), { target: { value: "Files to review" } });
    fireEvent.change(picker, {
      target: { files: [new File(["one"], "one.txt"), new File(["two"], "two.txt")] },
    });
    await waitFor(() => expect(container.querySelectorAll('[data-phase="done"]')).toHaveLength(2));
    fireEvent.click(screen.getByRole("button", { name: "Create post" }));
    await waitFor(() =>
      expect(create).toHaveBeenCalledWith(
        900,
        expect.objectContaining({
          brief: "",
          attachmentSignedId: null,
          attachmentSignedIds: ["signed-one.txt", "signed-two.txt"],
        }),
      ),
    );
    expect(screen.getByRole("button", { name: "Remove one.txt" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Remove two.txt" })).toBeTruthy();
  });

  it("refuses files above the same 100 MB policy as the room composer", () => {
    vi.spyOn(actions.boards, "postForm").mockResolvedValue({
      ownerCandidates: [],
      tagSuggestions: [],
      users: [],
      tags: [],
      tagsRequired: false,
      defaultBoardTagId: null,
    });
    const start = vi.spyOn(actions.messages, "startUpload");
    const large = new File(["bytes"], "large.bin");
    Object.defineProperty(large, "size", { value: 100 * 1024 * 1024 + 1 });

    const { container } = render(
      <NewPostDialog roomId={900} open onClose={() => undefined} onCreated={() => undefined} />,
    );

    const picker = container.querySelector<HTMLInputElement>('input[type="file"]');

    if (picker === null) throw new Error("no file input");
    fireEvent.change(picker, { target: { files: [large] } });
    expect(start).not.toHaveBeenCalled();
    expect(screen.queryByRole("list", { name: "Attachments" })).toBeNull();
    expect(toastSnapshot().at(-1)?.description).toBe(
      '"large.bin" exceeds the 100 MB upload limit.',
    );
  });

  it("doesn't open a post whose creation finishes after the room changed", async () => {
    let finish: (detail: ReturnType<typeof boardDetail>) => void = () => undefined;

    vi.spyOn(actions.boards, "postForm").mockResolvedValue({
      ownerCandidates: [],
      tagSuggestions: [],
      tags: [],
      tagsRequired: false,
      defaultBoardTagId: null,
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

  describe("with curated board tags", () => {
    const catalog = (tagsRequired: boolean, defaultBoardTagId: number | null) => {
      vi.spyOn(actions.boards, "postForm").mockResolvedValue({
        ownerCandidates: [],
        tagSuggestions: [],
        users: [],
        tags: [],
        tagsRequired: false,
        defaultBoardTagId: null,
      });
      vi.spyOn(actions.boards, "tagCatalog").mockImplementation(async (roomId) => {
        const answer = store.getState().boardTagCatalogs[roomId];

        if (answer === undefined) throw new Error("no catalog");

        return answer;
      });
      mutations.setBoardTagCatalog({
        roomId: 900,
        tags: [
          { id: 1, name: "Bug report", emoji: "🐞", position: 0 },
          { id: 2, name: "Idea", emoji: null, position: 1 },
        ],
        tagsRequired,
        defaultBoardTagId,
      });

      return vi.spyOn(actions.boards, "createPost").mockResolvedValue(boardDetail());
    };

    const chip = (name: string) => screen.getByRole("button", { name: new RegExp(name) });

    it("starts with the default picked and sends the picks with the free-text tags", async () => {
      const create = catalog(false, 2);
      const user = userEvent.setup();

      render(<NewPostDialog roomId={900} open onClose={() => {}} onCreated={() => {}} />);

      expect(chip("Idea").getAttribute("aria-pressed")).toBe("true");
      expect(chip("Bug report").getAttribute("aria-pressed")).toBe("false");
      expect(screen.getByText("Idea is the board's default.")).toBeTruthy();

      await user.click(chip("Bug report"));
      await user.type(screen.getByLabelText("Title"), "Crash on save");
      await user.type(screen.getByLabelText(/More tags/), "api");
      await user.click(screen.getByRole("button", { name: "Create post" }));

      await waitFor(() =>
        expect(create).toHaveBeenCalledWith(
          900,
          expect.objectContaining({ tags: ["Bug report", "Idea", "api"] }),
        ),
      );
    });

    it("holds the post until a required tag is picked, then names the problem and focuses the chips", async () => {
      const create = catalog(true, 2);
      const user = userEvent.setup();

      render(<NewPostDialog roomId={900} open onClose={() => {}} onCreated={() => {}} />);

      expect(screen.getByText("Required")).toBeTruthy();

      await user.click(chip("Idea"));
      await user.type(screen.getByLabelText("Title"), "Untagged");
      await user.click(screen.getByRole("button", { name: "Create post" }));

      expect(await screen.findByText("Pick at least one of this board's tags.")).toBeTruthy();
      expect(document.activeElement).toBe(chip("Bug report"));
      expect(create).not.toHaveBeenCalled();
      expect(screen.getByRole("group", { name: /Board tags/ }).getAttribute("aria-invalid")).toBe(
        "true",
      );

      await user.click(chip("Bug report"));
      await user.click(screen.getByRole("button", { name: "Create post" }));

      await waitFor(() =>
        expect(create).toHaveBeenCalledWith(900, expect.objectContaining({ tags: ["Bug report"] })),
      );
    });

    it("follows a live catalog change: a deleted default stops being picked", async () => {
      catalog(false, 2);

      render(<NewPostDialog roomId={900} open onClose={() => {}} onCreated={() => {}} />);

      expect(chip("Idea").getAttribute("aria-pressed")).toBe("true");

      act(() =>
        mutations.setBoardTagCatalog({
          roomId: 900,
          tags: [{ id: 1, name: "Bug report", emoji: "🐞", position: 0 }],
          tagsRequired: false,
          defaultBoardTagId: null,
        }),
      );

      expect(screen.queryByRole("button", { name: /Idea/ })).toBeNull();
      expect(chip("Bug report").getAttribute("aria-pressed")).toBe("false");
    });
  });
});
