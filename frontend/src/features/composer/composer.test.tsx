import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { emptyTimeline, initialState } from "../../store/state.ts";
import { store } from "../../store/store.ts";
import { composerActions } from "../../sync/composer-actions.ts";
import { actions } from "../../sync/runtime.ts";
import { clearCommandCache } from "./autocomplete/suggestions.ts";
import { Composer } from "./composer.tsx";
import { draftKey, readDraft } from "./draft.ts";

const ROOM = 12;

function serveDrive() {
  vi.spyOn(actions.drive, "search").mockResolvedValue({
    files: ["Roadmap", "Budget"].map((name) => ({
      id: name.toLowerCase(),
      name,
      kind: "document",
      modifiedAt: null,
      owner: null,
      url: null,
    })),
  });
  vi.spyOn(actions.drive, "recipients").mockResolvedValue({ recipients: [] });
}

async function attachDrive(name: string) {
  fireEvent.click(screen.getByRole("button", { name: "Attach and more" }));
  fireEvent.click(await screen.findByRole("menuitem", { name: "From Google Drive" }));
  fireEvent.click(await screen.findByRole("option", { name: new RegExp(name) }));
  fireEvent.click(await screen.findByRole("button", { name: "Attach only" }));
  await screen.findByRole("button", { name: `Remove ${name}` });
}

beforeEach(() => {
  sessionStorage.clear();
  clearCommandCache();
  store.setState(
    {
      ...initialState,
      timelines: { [ROOM]: { ...emptyTimeline, status: "ready", after: 9 } },
    },
    true,
  );
  vi.stubGlobal("matchMedia", (query: string) =>
    Object.assign(new EventTarget(), {
      matches: true,
      media: query,
      onchange: null,
    }),
  );
  vi.spyOn(actions, "noteActivity").mockImplementation(() => undefined);
  vi.spyOn(actions, "setTyping").mockImplementation(() => undefined);
  vi.spyOn(actions.scheduled, "load").mockResolvedValue(undefined);
  vi.spyOn(composerActions, "slashCommands").mockResolvedValue([]);
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  store.setState(initialState, true);
});

describe("sending from history", () => {
  it("posts an ordinary message immediately while the latest-page fetch is pending", async () => {
    const latest = Promise.withResolvers<void>();
    const jump = vi.spyOn(actions, "jumpToPresent").mockReturnValue(latest.promise);
    const send = vi.spyOn(actions, "send").mockImplementation(() => undefined);

    render(<Composer roomId={ROOM} placeholder="Message" />);
    const input = screen.getByRole("textbox", { name: "Message" });

    fireEvent.change(input, { target: { value: "First message" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(jump).toHaveBeenCalledWith(ROOM);
    expect(send).toHaveBeenCalledExactlyOnceWith(ROOM, "First message", {
      threadId: null,
      attachmentSignedId: null,
      attachment: null,
      reply: null,
      clientMessageId: expect.any(String),
    });
    expect(input).toHaveProperty("value", "");
    fireEvent.change(input, { target: { value: "My next message" } });
    expect(screen.getByRole("button", { name: "Send message" })).toHaveProperty("disabled", false);

    await act(async () => latest.resolve());

    expect(input).toHaveProperty("value", "My next message");
    expect(readDraft(draftKey(ROOM, null))).toBe("My next message");
  });

  it.each(["/play bell"])(
    "preserves a new draft typed while %s awaits the latest page",
    async (submitted) => {
      const latest = Promise.withResolvers<void>();
      const jump = vi.spyOn(actions, "jumpToPresent").mockReturnValue(latest.promise);
      const send = vi.spyOn(actions, "send").mockImplementation(() => undefined);

      render(<Composer roomId={ROOM} placeholder="Message" />);
      const input = screen.getByRole("textbox", { name: "Message" });

      fireEvent.change(input, { target: { value: submitted } });
      fireEvent.keyDown(input, { key: "Enter" });
      expect(jump).toHaveBeenCalledWith(ROOM);
      expect(send).not.toHaveBeenCalled();
      fireEvent.change(input, { target: { value: "My next message" } });

      await act(async () => latest.resolve());

      expect(send).toHaveBeenCalledExactlyOnceWith(ROOM, submitted.trimEnd(), {
        threadId: null,
        attachmentSignedId: null,
        attachment: null,
        reply: null,
        clientMessageId: expect.any(String),
      });
      expect(input).toHaveProperty("value", "My next message");
      expect(readDraft(draftKey(ROOM, null))).toBe("My next message");
    },
  );

  it("clears the captured draft when the reader has not edited it", async () => {
    const latest = Promise.withResolvers<void>();

    vi.spyOn(actions, "jumpToPresent").mockReturnValue(latest.promise);
    vi.spyOn(actions, "send").mockImplementation(() => undefined);
    render(<Composer roomId={ROOM} placeholder="Message" />);
    const input = screen.getByRole("textbox", { name: "Message" });

    fireEvent.change(input, { target: { value: "First message  " } });
    fireEvent.keyDown(input, { key: "Enter" });
    await act(async () => latest.resolve());
    expect(input).toHaveProperty("value", "");
    expect(readDraft(draftKey(ROOM, null))).toBe("");
  });
});

describe("a send waiting on uploads", () => {
  it("sends only what was submitted, keeping text and files added while it waited", async () => {
    const firstUpload = Promise.withResolvers<{ signedId: string; uploadUrl: string }>();

    serveDrive();

    vi.spyOn(actions, "jumpToPresent").mockResolvedValue(undefined);

    const send = vi.spyOn(actions, "send").mockImplementation(() => undefined);

    vi.spyOn(actions.messages, "startUpload").mockImplementation((body) =>
      body.filename === "first.txt"
        ? firstUpload.promise
        : Promise.resolve({ signedId: `signed-${body.filename}`, uploadUrl: "/put" }),
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

    const { container } = render(<Composer roomId={ROOM} placeholder="Message" />);
    const input = screen.getByRole("textbox", { name: "Message" });
    const picker = container.querySelector<HTMLInputElement>('input[type="file"]');

    if (picker === null) {
      throw new Error("no file input");
    }

    const attach = (name: string) =>
      act(async () => {
        fireEvent.change(picker, { target: { files: [new File(["bytes"], name)] } });
      });

    fireEvent.change(input, { target: { value: "With the file" } });
    await attach("first.txt");
    await attachDrive("Roadmap");
    fireEvent.keyDown(input, { key: "Enter" });
    expect(send).not.toHaveBeenCalled();

    fireEvent.change(input, { target: { value: "Typed while waiting" } });
    await attach("second.txt");
    await attachDrive("Budget");
    await waitFor(() => expect(screen.getByText("second.txt")).toBeTruthy());
    expect(send).not.toHaveBeenCalled();

    await act(async () => firstUpload.resolve({ signedId: "signed-first", uploadUrl: "/put" }));

    await waitFor(() => expect(send).toHaveBeenCalledTimes(1));
    expect(send).toHaveBeenCalledWith(
      ROOM,
      "With the file",
      expect.objectContaining({ attachmentSignedId: "signed-first", driveFileIds: ["roadmap"] }),
    );
    expect(input).toHaveProperty("value", "Typed while waiting");
    expect(screen.queryByText("first.txt")).toBeNull();
    expect(screen.getByText("second.txt")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Remove Roadmap" })).toBeNull();
    expect(screen.getByRole("button", { name: "Remove Budget" })).toBeTruthy();
  });
});

describe("Drive submissions", () => {
  it("sends a Drive file without text while the latest page is loading", async () => {
    const latest = Promise.withResolvers<void>();

    serveDrive();
    vi.spyOn(actions, "jumpToPresent").mockReturnValue(latest.promise);
    const send = vi.spyOn(actions, "send").mockImplementation(() => undefined);

    render(<Composer roomId={ROOM} placeholder="Message" />);
    await attachDrive("Roadmap");
    fireEvent.keyDown(screen.getByRole("textbox", { name: "Message" }), { key: "Enter" });

    expect(send).toHaveBeenCalledExactlyOnceWith(
      ROOM,
      "",
      expect.objectContaining({ driveFileIds: ["roadmap"], reply: null }),
    );
    expect(screen.queryByRole("button", { name: "Remove Roadmap" })).toBeNull();
    await act(async () => latest.resolve());
  });

  it("keeps a Drive file added while a new thread's first send is in flight", async () => {
    const sent = Promise.withResolvers<void>();
    const submit = vi.fn(() => sent.promise);

    serveDrive();
    render(<Composer roomId={ROOM} placeholder="Message" onSubmit={submit} />);
    const input = screen.getByRole("textbox", { name: "Message" });

    fireEvent.change(input, { target: { value: "First reply" } });
    await attachDrive("Roadmap");
    fireEvent.keyDown(input, { key: "Enter" });
    expect(submit).toHaveBeenCalledExactlyOnceWith({
      markdown: "First reply",
      attachmentSignedId: null,
      driveFileIds: ["roadmap"],
    });
    await attachDrive("Budget");
    await act(async () => sent.resolve());

    expect(input).toHaveProperty("value", "");
    expect(screen.queryByRole("button", { name: "Remove Roadmap" })).toBeNull();
    expect(screen.getByRole("button", { name: "Remove Budget" })).toBeTruthy();
  });
});

describe("a slash command that fails", () => {
  const remind = {
    name: "remind",
    description: "Set a reminder",
    argHint: "<when> <text>",
    takesArguments: true,
    agentName: null,
  };

  it.each([
    ["errors", () => Promise.resolve({ status: "error" as const, message: "Bad time" })],
    ["can't run", () => Promise.reject(new Error("Offline"))],
  ])("keeps a draft typed while it ran when it %s", async (_label, outcome) => {
    const gate = Promise.withResolvers<void>();

    vi.spyOn(composerActions, "slashCommands").mockResolvedValue([remind]);

    const run = vi
      .spyOn(composerActions, "runSlashCommand")
      .mockImplementation(() => gate.promise.then(outcome));

    render(<Composer roomId={ROOM} placeholder="Message" />);
    const input = screen.getByRole("textbox", { name: "Message" });

    fireEvent.change(input, { target: { value: "/remind tomorrow lunch" } });
    fireEvent.keyDown(input, { key: "Enter" });
    await waitFor(() => expect(run).toHaveBeenCalledTimes(1));
    expect(input).toHaveProperty("value", "");

    fireEvent.change(input, { target: { value: "Something newer" } });
    await act(async () => gate.resolve());

    expect(input).toHaveProperty("value", "Something newer");
    expect(readDraft(draftKey(ROOM, null))).toBe("Something newer");
  });

  it("puts the command back when nothing was typed while it ran", async () => {
    vi.spyOn(composerActions, "slashCommands").mockResolvedValue([remind]);
    vi.spyOn(composerActions, "runSlashCommand").mockResolvedValue({
      status: "error",
      message: "Bad time",
    });

    render(<Composer roomId={ROOM} placeholder="Message" />);
    const input = screen.getByRole("textbox", { name: "Message" });

    fireEvent.change(input, { target: { value: "/remind tomorrow lunch" } });
    fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() => expect(input).toHaveProperty("value", "/remind tomorrow lunch"));
  });
});
