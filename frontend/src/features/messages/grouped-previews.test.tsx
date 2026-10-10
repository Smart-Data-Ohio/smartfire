import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SEED_IDS } from "../../../mock/server.ts";
import { messageFixture, pageFixture } from "../../api/testing.ts";
import type { Attachment } from "../../gen/Attachment.ts";
import { mutations } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { installMockNetwork, type MockNetwork } from "../../test/mock-network.ts";
import { ReplyChip } from "../composer/reply-chip.tsx";
import { PinsPane } from "../panes/pins-pane.tsx";
import { SavedRow } from "../saved/saved-row.tsx";
import { HitRow } from "../search/hit-row.tsx";
import DeleteDialog from "./delete-dialog.tsx";
import ForwardDialog from "./forward-dialog.tsx";
import { ReplyQuote, snippet } from "./message-content.tsx";
import { MessageEditor } from "./message-editor.tsx";

const ROOM = SEED_IDS.rooms.quiet;

const files: Attachment[] = ["first.txt", "second.pdf", "third.png"].map((filename) => ({
  filename,
  contentType: "text/plain",
  byteSize: 12,
  width: null,
  height: null,
  preview: "file",
  url: `/files/${filename}`,
  downloadUrl: `/files/${filename}?disposition=attachment`,
  thumbnailUrl: null,
}));

const grouped = () =>
  messageFixture(21, ROOM, {
    creatorId: SEED_IDS.viewer,
    bodyHtml: "",
    markdownSource: "",
    attachment: files[0] ?? null,
    attachments: files,
  });

let network: MockNetwork;

beforeEach(() => {
  network = installMockNetwork();
  mutations.reset();
  vi.stubGlobal("matchMedia", (query: string) =>
    Object.assign(new EventTarget(), { matches: false, media: query, onchange: null }),
  );
  Element.prototype.scrollIntoView = () => undefined;

  const meta = document.createElement("meta");
  meta.name = "csrf-token";
  meta.content = network.server.csrfToken();
  document.head.append(meta);
});

afterEach(() => {
  network.restore();
  mutations.reset();
  document.head.querySelector('meta[name="csrf-token"]')?.remove();
});

async function mount(children: ReactNode) {
  const root = createRootRoute({ component: () => children });

  const router = createRouter({
    routeTree: root.addChildren([
      createRoute({ getParentRoute: () => root, path: "/r/$roomId/m/$messageId" }),
      createRoute({ getParentRoute: () => root, path: "/r/$roomId/t/$threadId" }),
    ]),
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });

  const result = render(<RouterProvider router={router} />);
  await act(() => router.load());

  return result;
}

describe("grouped file previews", () => {
  it("quotes the file count, including captioned messages, and keeps legacy filenames", () => {
    expect(snippet(grouped())).toBe("3 files");
    expect(snippet({ ...grouped(), markdownSource: "Here are the plans" })).toBe(
      "Here are the plans · 3 files",
    );
    expect(
      snippet(
        messageFixture(1, ROOM, { markdownSource: "", bodyHtml: "", attachment: files[0] ?? null }),
      ),
    ).toBe("first.txt");
  });

  it("shows the count in reply quotes and the composer's reply chip", async () => {
    const source = grouped();
    mutations.applyPage(ROOM, pageFixture([source]), "replace");
    await mount(
      <>
        <ReplyQuote message={messageFixture(22, ROOM, { replyToMessageId: source.id })} />
        <ReplyChip
          target={{ messageId: source.id, notify: true, seq: 1 }}
          onNotifyChange={() => {}}
          onCancel={() => {}}
        />
      </>,
    );
    expect(screen.getAllByText("3 files")).toHaveLength(2);
  });

  it("names every file in a search hit", async () => {
    const { container } = await mount(
      <HitRow hit={grouped()} conversation={undefined} terms={[]} now={Date.now()} />,
    );

    expect(
      [...container.querySelectorAll(".search-hit-file-name")].map((node) => node.textContent),
    ).toEqual(["first.txt", "second.pdf", "third.png"]);
  });

  it("names every file in saved items", () => {
    const { container } = render(
      <SavedRow
        item={{
          id: 1,
          messageId: 21,
          status: "in_progress",
          remindAt: null,
          remindedAt: null,
          createdAt: "2026-10-06T09:00:00.000Z",
        }}
        message={grouped()}
        conversation={null}
        now={Date.now()}
        motion={undefined}
        celebrate={false}
        handlers={{
          onOpen: () => {},
          onToggleDone: () => {},
          onRemind: () => {},
          onCustomRemind: () => {},
          onRemove: () => {},
          onMenu: () => {},
        }}
      />,
    );

    expect(
      [...container.querySelectorAll(".saved-chip-text")].map((node) => node.textContent),
    ).toEqual(["first.txt", "second.pdf", "third.png"]);
  });

  it("names every file in the pins pane", async () => {
    const sent = network.server.post(ROOM, SEED_IDS.viewer, "Grouped pin");
    sent.attachment = files[0] ?? null;
    sent.attachments = files;
    network.server.pin(sent.id, SEED_IDS.viewer);

    const { container } = await mount(<PinsPane roomId={ROOM} />);
    await screen.findByText("Grouped pin");

    expect(
      [...container.querySelectorAll(".pin-card-file")].map((node) => node.textContent),
    ).toEqual(["first.txt", "second.pdf", "third.png"]);
  });

  it("shows the file count in delete confirmation", async () => {
    render(<DeleteDialog message={grouped()} open onOpenChange={() => {}} onConfirm={() => {}} />);
    const dialog = await screen.findByRole("alertdialog", { name: "Delete message?" });
    expect(within(dialog).getByText("3 files")).toBeTruthy();
  });

  it("shows the file count in forwarding", async () => {
    render(<ForwardDialog message={grouped()} open onOpenChange={() => {}} />);
    const dialog = await screen.findByRole("dialog", { name: "Forward message" });
    expect(within(dialog).getByText("3 files")).toBeTruthy();
  });

  it("clears a grouped message's caption without requesting deletion", async () => {
    const user = userEvent.setup();
    const sent = network.server.post(ROOM, SEED_IDS.viewer, "Caption");
    sent.attachment = null;
    sent.attachments = files;
    const onClose = vi.fn();
    const onRequestDelete = vi.fn();

    render(<MessageEditor message={sent} onClose={onClose} onRequestDelete={onRequestDelete} />);
    await user.clear(screen.getByRole("textbox", { name: "Edit message" }));
    await user.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(onClose).toHaveBeenCalledOnce());
    expect(onRequestDelete).not.toHaveBeenCalled();

    const saved = await actions.messages.read(sent.id);

    expect(saved.message.markdownSource).toBe("");
    expect(saved.message.attachments).toEqual(files);
  });
});
