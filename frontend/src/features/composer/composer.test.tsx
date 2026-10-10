import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { userFixture } from "../../api/testing.ts";
import type { Thread } from "../../gen/Thread.ts";
import { emptyTimeline, initialState } from "../../store/state.ts";
import { store } from "../../store/store.ts";
import { composerActions } from "../../sync/composer-actions.ts";
import { actions } from "../../sync/runtime.ts";
import { toastSnapshot } from "../../ui/toast-store.ts";
import { clearCommandCache } from "./autocomplete/suggestions.ts";
import { Composer } from "./composer.tsx";
import { draftKey, readDraft } from "./draft.ts";

const ROOM = 12;

function pollThread(status: Thread["status"]): Thread {
  return {
    id: 7,
    roomId: ROOM,
    parentMessageId: null,
    creatorId: 1,
    name: "Lunch",
    status,
    replyCount: 0,
    lastActivityAt: "2026-10-09T12:00:00Z",
    autoArchiveAfterMinutes: 1440,
    createdAt: "2026-10-09T12:00:00Z",
    work: null,
  };
}

function serveDrive() {
  vi.spyOn(actions.drive, "preparePicker").mockResolvedValue({
    choose: async () => null,
    dispose: () => {},
  });
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
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  store.setState(initialState, true);
});

it("refuses files above the workspace limit before adding them to the attachment tray", () => {
  store.setState({
    boot: {
      user: { id: 1, name: "David", avatarUrl: "/avatar" },
      account: {
        name: "Smartfire",
        logoUrl: null,
        logoStillUrl: null,
        bannerUrl: null,
        bannerStillUrl: null,
        uploadLimitBytes: 1024 * 1024,
      },
      customStyles: null,
      theme: "system",
      textSize: "default",
      cableUrl: "/cable",
      serviceWorkerUrl: null,
      version: "test",
      revision: null,
      appearancePreferences: null,
    },
  });
  const start = vi.spyOn(actions.messages, "startUpload");
  const large = new File(["bytes"], "large.bin");
  Object.defineProperty(large, "size", { value: 1024 * 1024 + 1 });
  const { container } = render(<Composer roomId={ROOM} />);
  const picker = container.querySelector<HTMLInputElement>('input[type="file"]');

  if (picker === null) throw new Error("no file input");
  fireEvent.change(picker, { target: { files: [large] } });
  expect(start).not.toHaveBeenCalled();
  expect(screen.queryByText("large.bin")).toBeNull();
  expect(toastSnapshot().at(-1)?.description).toBe('"large.bin" exceeds the 1 MB upload limit.');
});

describe("polls in the composer", () => {
  it("posts a poll to the active thread and preserves its draft on refusal", async () => {
    store.setState({ threads: { 7: pollThread("active") } });

    const create = vi
      .spyOn(actions.cards, "createPoll")
      .mockRejectedValue(new Error("Thread closed"));

    render(<Composer roomId={ROOM} threadId={7} placeholder="Message" />);
    fireEvent.click(screen.getByRole("button", { name: "Attach and more" }));
    fireEvent.click(await screen.findByRole("menuitem", { name: "Create a poll" }));
    await screen.findByRole("dialog", { name: "Create a poll" });
    fireEvent.change(screen.getByRole("textbox", { name: "Question" }), {
      target: { value: "Lunch?" },
    });
    fireEvent.change(screen.getByRole("textbox", { name: "Option 1" }), {
      target: { value: "Pizza" },
    });
    fireEvent.change(screen.getByRole("textbox", { name: "Option 2" }), {
      target: { value: "Tacos" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Post poll" }));
    await screen.findByText("Thread closed");
    expect(create).toHaveBeenCalledExactlyOnceWith(ROOM, {
      clientMessageId: expect.any(String),
      question: "Lunch?",
      options: ["Pizza", "Tacos"],
      multiple: false,
      anonymous: false,
      closesAt: null,
      threadId: 7,
    });
    expect(screen.getByRole("textbox", { name: "Question" })).toHaveProperty("value", "Lunch?");
  });

  it.each(["closed", "locked"] as const)("hides poll creation in a %s thread", async (status) => {
    store.setState({ threads: { 7: pollThread(status) } });
    render(<Composer roomId={ROOM} threadId={7} />);
    fireEvent.click(screen.getByRole("button", { name: "Attach and more" }));
    await screen.findByRole("menuitem", { name: "Upload a file" });
    expect(screen.queryByRole("menuitem", { name: "Create a poll" })).toBeNull();
  });

  it("keeps poll creation out of a new thread's first reply", async () => {
    render(<Composer roomId={ROOM} onSubmit={async () => {}} />);
    fireEvent.click(screen.getByRole("button", { name: "Attach and more" }));
    await screen.findByRole("menuitem", { name: "Upload a file" });
    expect(screen.queryByRole("menuitem", { name: "Create a poll" })).toBeNull();
  });
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

describe("mention autocomplete", () => {
  it("inserts and sends the selected ID when two people have the same name", async () => {
    Element.prototype.scrollIntoView = () => undefined;
    vi.spyOn(composerActions, "suggestUsers").mockResolvedValue([
      { user: userFixture(123, "Twin"), mentionToken: "<@123>" },
      { user: userFixture(456, "Twin"), mentionToken: "<@456>" },
    ]);
    vi.spyOn(actions, "jumpToPresent").mockResolvedValue(undefined);
    const send = vi.spyOn(actions, "send").mockImplementation(() => undefined);

    render(<Composer roomId={ROOM} placeholder="Message" />);
    const input = screen.getByRole("textbox", { name: "Message" });

    fireEvent.focus(input);
    fireEvent.change(input, { target: { value: "Hello @tw" } });
    const options = await screen.findAllByRole("option", { name: "Twin" });
    const second = options[1];

    expect(options).toHaveLength(2);

    if (second === undefined) {
      throw new Error("the second Twin is missing");
    }

    expect(second.getAttribute("aria-disabled")).not.toBe("true");
    fireEvent.click(second);
    expect(input).toHaveProperty("value", "Hello <@456> ");
    fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() =>
      expect(send).toHaveBeenCalledWith(ROOM, "Hello <@456>", expect.any(Object)),
    );
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

describe("a Drive grant finishing after send", () => {
  it("adds only the new file to the next draft after the submitted upload finishes", async () => {
    const upload = Promise.withResolvers<{ signedId: string; uploadUrl: string }>();
    const granted = Promise.withResolvers<Awaited<ReturnType<typeof actions.drive.share>>>();
    serveDrive();
    vi.spyOn(actions.drive, "recipients").mockResolvedValue({
      recipients: [{ id: 2, name: "Maya", email: "maya@example.com" }],
    });
    vi.spyOn(actions.drive, "share").mockReturnValue(granted.promise);
    vi.spyOn(actions.messages, "startUpload").mockReturnValue(upload.promise);
    vi.spyOn(actions, "jumpToPresent").mockResolvedValue(undefined);
    const send = vi.spyOn(actions, "send").mockImplementation(() => undefined);
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

    if (picker === null) throw new Error("no file input");
    fireEvent.change(picker, { target: { files: [new File(["bytes"], "pending.txt")] } });
    await attachDrive("Roadmap");
    fireEvent.change(input, { target: { value: "Submitted with A" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(send).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Attach and more" }));
    fireEvent.click(await screen.findByRole("menuitem", { name: "From Google Drive" }));
    fireEvent.click(await screen.findByRole("option", { name: /Budget/ }));
    fireEvent.click(await screen.findByRole("checkbox", { name: /Maya/ }));
    fireEvent.click(screen.getByRole("button", { name: "Grant view access and attach" }));
    await act(async () => upload.resolve({ signedId: "signed-pending", uploadUrl: "/put" }));
    await waitFor(() => expect(send).toHaveBeenCalledTimes(1));
    expect(send).toHaveBeenCalledWith(
      ROOM,
      "Submitted with A",
      expect.objectContaining({ driveFileIds: ["roadmap"] }),
    );
    expect(screen.queryByRole("button", { name: "Remove Roadmap" })).toBeNull();
    await act(async () =>
      granted.resolve({
        outcome: "shared",
        fileId: "budget",
        blocked: null,
        changedIds: [],
        recipients: [],
        results: [],
      }),
    );
    expect(await screen.findByRole("button", { name: "Remove Budget" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Remove Roadmap" })).toBeNull();
    fireEvent.keyDown(input, { key: "Enter" });
    expect(send).toHaveBeenLastCalledWith(
      ROOM,
      "",
      expect.objectContaining({ driveFileIds: ["budget"] }),
    );
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

describe("several files in one send", () => {
  function landUploads() {
    vi.spyOn(actions.messages, "startUpload").mockImplementation((body) =>
      Promise.resolve({ signedId: `signed-${body.filename}`, uploadUrl: "/put" }),
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
  }

  function picker(container: HTMLElement): HTMLInputElement {
    const input = container.querySelector<HTMLInputElement>('input[type="file"]');

    if (input === null) throw new Error("no file input");

    return input;
  }

  it("posts the text and every file as one message", async () => {
    landUploads();
    vi.spyOn(actions, "jumpToPresent").mockResolvedValue(undefined);
    const send = vi.spyOn(actions, "send").mockImplementation(() => undefined);
    const { container } = render(<Composer roomId={ROOM} placeholder="Message" />);
    const input = screen.getByRole("textbox", { name: "Message" });

    await act(async () => {
      fireEvent.change(picker(container), {
        target: { files: ["a.txt", "b.txt", "c.txt"].map((name) => new File(["bytes"], name)) },
      });
    });
    fireEvent.change(input, { target: { value: "Three files" } });
    await waitFor(() => expect(screen.getAllByText(/TXT · 5 bytes/)).toHaveLength(3));
    fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() => expect(send).toHaveBeenCalledTimes(1));
    expect(send).toHaveBeenCalledWith(
      ROOM,
      "Three files",
      expect.objectContaining({
        attachmentSignedId: null,
        attachmentSignedIds: ["signed-a.txt", "signed-b.txt", "signed-c.txt"],
        attachments: [
          expect.objectContaining({ filename: "a.txt" }),
          expect.objectContaining({ filename: "b.txt" }),
          expect.objectContaining({ filename: "c.txt" }),
        ],
      }),
    );
    expect(screen.queryByRole("list", { name: "Attachments" })).toBeNull();
  });

  it("holds the send while a file has failed, until it's removed", async () => {
    landUploads();
    vi.spyOn(actions.messages, "startUpload").mockImplementation((body) =>
      body.filename === "bad.txt"
        ? Promise.reject(new Error("Network down"))
        : Promise.resolve({ signedId: `signed-${body.filename}`, uploadUrl: "/put" }),
    );
    vi.spyOn(actions, "jumpToPresent").mockResolvedValue(undefined);
    const send = vi.spyOn(actions, "send").mockImplementation(() => undefined);
    const { container } = render(<Composer roomId={ROOM} placeholder="Message" />);
    const input = screen.getByRole("textbox", { name: "Message" });

    await act(async () => {
      fireEvent.change(picker(container), {
        target: { files: ["good.txt", "bad.txt"].map((name) => new File(["bytes"], name)) },
      });
    });
    await screen.findByRole("button", { name: "Retry bad.txt" });
    fireEvent.keyDown(input, { key: "Enter" });

    expect(send).not.toHaveBeenCalled();
    expect(toastSnapshot().at(-1)?.title).toBe("A file didn't upload");

    fireEvent.click(screen.getByRole("button", { name: "Remove bad.txt" }));
    fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() => expect(send).toHaveBeenCalledTimes(1));
    expect(send).toHaveBeenCalledWith(
      ROOM,
      "",
      expect.objectContaining({ attachmentSignedId: "signed-good.txt" }),
    );
  });

  it("says how many files didn't fit under the cap", async () => {
    landUploads();
    const { container } = render(<Composer roomId={ROOM} placeholder="Message" />);

    await act(async () => {
      fireEvent.change(picker(container), {
        target: {
          files: Array.from({ length: 12 }, (_, index) => new File(["bytes"], `f${index}.txt`)),
        },
      });
    });

    expect(screen.getAllByRole("listitem")).toHaveLength(10);
    expect(toastSnapshot().at(-1)).toMatchObject({
      title: "A message holds up to 10 files",
      description: "2 files weren't added. Send these, then add the rest.",
    });
  });
});

it("schedules two uploaded files in tray order without requiring text", async () => {
  vi.spyOn(actions.messages, "startUpload").mockImplementation(async (body) => ({
    signedId: `signed-${body.filename}`,
    uploadUrl: "/put",
  }));
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
  const create = vi.spyOn(actions.scheduled, "create").mockRejectedValue(new Error("Keep draft"));
  const { container } = render(<Composer roomId={ROOM} />);
  const picker = container.querySelector<HTMLInputElement>('input[type="file"]');

  if (picker === null) throw new Error("no file input");
  fireEvent.change(picker, {
    target: { files: [new File(["one"], "one.txt"), new File(["two"], "two.txt")] },
  });
  await waitFor(() => expect(container.querySelectorAll('[data-phase="done"]')).toHaveLength(2));
  fireEvent.click(screen.getByRole("button", { name: "Schedule message" }));
  fireEvent.click(await screen.findByRole("menuitem", { name: /In 1 hour/ }));
  await waitFor(() =>
    expect(create).toHaveBeenCalledWith(
      ROOM,
      expect.objectContaining({
        markdownSource: "",
        attachmentSignedIds: ["signed-one.txt", "signed-two.txt"],
      }),
    ),
  );
  expect(screen.getByRole("button", { name: "Remove one.txt" })).toBeDefined();
});
