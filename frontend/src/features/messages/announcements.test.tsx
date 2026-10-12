import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { messageFixture } from "../../api/testing.ts";
import { mutations } from "../../store/store.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { removeToast, toastSnapshot } from "../../ui/toast-store.ts";
import { publish } from "./commands.ts";
import { AnnouncementOrigin, PublishedMark } from "./message-content.tsx";
import { menuSections } from "./message-menu.tsx";
import { type MessageContext, messagePermissions } from "./permissions.ts";

const NEWS = 5;

const author: MessageContext = {
  viewerId: 7,
  viewerRole: "member",
  roomKind: "open",
  threadStatus: null,
  announcement: true,
};

const own = messageFixture(1, NEWS, { creatorId: 7 });

const theirs = messageFixture(2, NEWS, { creatorId: 9 });

const source = { roomId: NEWS, messageId: 1, roomName: "news" };

afterEach(() => {
  vi.restoreAllMocks();
  mutations.reset();

  for (const record of [...toastSnapshot()]) removeToast(record.id);
});

describe("publish permission", () => {
  it("is the author's or an administrator's, in an announcement room", () => {
    expect(messagePermissions(own, author).publish).toBe(true);
    expect(messagePermissions(theirs, author).publish).toBe(false);
    expect(messagePermissions(theirs, { ...author, viewerRole: "administrator" }).publish).toBe(
      true,
    );
    expect(messagePermissions(own, { ...author, announcement: false }).publish).toBe(false);
    // Before the room's flag is known, nothing is offered.
    const { announcement: _, ...unknown } = author;

    expect(messagePermissions(own, unknown).publish).toBe(false);
    expect(messagePermissions(own, { ...author, viewerRole: "bot" }).publish).toBe(false);
  });

  it("isn't offered on a copy whose source the viewer can't see", () => {
    // An administrator outside the source room: the server leaves the source out but still marks
    // the copy, here in a room that is itself an announcement room.
    const hidden = messageFixture(6, NEWS, {
      creatorId: 9,
      announcementCopy: true,
      announcementSource: null,
    });

    const admin = { ...author, viewerRole: "administrator" } as const;

    expect(messagePermissions(hidden, admin).publish).toBe(false);
    expect(
      menuSections({
        permissions: messagePermissions(hidden, admin),
        pinned: false,
        published: false,
        saved: false,
        inThread: false,
      })
        .flat()
        .some((entry) => entry.command === "publish"),
    ).toBe(false);
  });

  it("is never offered on a system note, a streaming message or a delivered copy", () => {
    const admin = { ...author, viewerRole: "administrator" } as const;

    expect(
      messagePermissions(messageFixture(3, NEWS, { creatorId: 7, systemNote: true }), admin)
        .publish,
    ).toBe(false);
    expect(
      messagePermissions(messageFixture(4, NEWS, { creatorId: 7, streaming: true }), admin).publish,
    ).toBe(false);
    expect(
      messagePermissions(
        messageFixture(5, NEWS, { creatorId: 7, announcementSource: source }),
        admin,
      ).publish,
    ).toBe(false);
  });
});

describe("publish in the message menu", () => {
  const publishEntry = (published: boolean, context: MessageContext = author) =>
    menuSections({
      permissions: messagePermissions(own, context),
      pinned: false,
      published,
      saved: false,
      inThread: false,
    })
      .flat()
      .find((entry) => entry.command === "publish");

  it("offers Publish, then shows Published and can't be chosen again", () => {
    expect(publishEntry(false)).toMatchObject({ label: "Publish", icon: "megaphone" });
    expect(publishEntry(false)?.disabled).toBeUndefined();
    expect(publishEntry(true)).toMatchObject({ label: "Published", disabled: true });
  });

  it("leaves the entry out for someone who may not publish", () => {
    expect(publishEntry(false, { ...author, announcement: false })).toBeUndefined();
    expect(publishEntry(true, { ...author, viewerId: 9 })).toBeUndefined();
  });

  it("publishes through the API and says how it went", async () => {
    const call = vi
      .spyOn(actions.announcements, "publish")
      .mockResolvedValue({ ...own, published: true });

    publish(own);

    expect(call).toHaveBeenCalledWith(1);
    await waitFor(() =>
      expect(toastSnapshot().map((record) => record.title)).toEqual([
        "Published to the channels that follow this one",
      ]),
    );
  });

  it("shows the server's refusal", async () => {
    vi.spyOn(actions.announcements, "publish").mockRejectedValue(
      new ActionError(
        "Validation",
        "Message cannot publish a copy, system note or unfinished message",
        {
          message: ["cannot publish a copy, system note or unfinished message"],
        },
      ),
    );

    publish(own);

    await waitFor(() =>
      expect(toastSnapshot()).toMatchObject([
        {
          title: "Couldn't publish the message",
          description: "Message cannot publish a copy, system note or unfinished message",
          tone: "danger",
        },
      ]),
    );
  });
});

async function inRouter(children: ReactNode) {
  const root = createRootRoute();

  const room = createRoute({
    getParentRoute: () => root,
    path: "/r/$roomId",
    component: () => children,
  });

  const message = createRoute({
    getParentRoute: () => root,
    path: "/r/$roomId/m/$messageId",
    component: () => null,
  });

  const router = createRouter({
    routeTree: root.addChildren([room, message]),
    history: createMemoryHistory({ initialEntries: ["/r/3"] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());
}

describe("published marks and copy attribution", () => {
  it("marks a published message and nothing else", () => {
    const { container, rerender } = render(<PublishedMark published />);

    expect(container.textContent).toBe("Published");
    rerender(<PublishedMark published={false} />);
    expect(container.textContent).toBe("");
  });

  it("names the source room and workspace, linking to the original", async () => {
    mutations.setBoot({
      user: { id: 7, name: "Ada Lovelace", avatarUrl: "/avatar" },
      account: {
        name: "Smart Data",
        logoUrl: null,
        logoStillUrl: null,
        bannerUrl: null,
        bannerStillUrl: null,
        uploadLimitBytes: 100 * 1024 * 1024,
      },
      customStyles: null,
      theme: "system",
      textSize: "default",
      cableUrl: "/cable",
      serviceWorkerUrl: null,
      version: "test",
      revision: null,
      appearancePreferences: null,
    });

    await inRouter(
      <AnnouncementOrigin source={{ roomId: NEWS, messageId: 41, roomName: "news" }} />,
    );

    const link = screen.getByRole("link");

    expect(link.textContent).toBe("#news · Smart Data");
    expect(link.getAttribute("href")).toBe(`/r/${NEWS}/m/41`);
  });

  it("shows nothing when the server leaves the source out", async () => {
    await inRouter(
      <div data-testid="row">
        <AnnouncementOrigin source={null} />
      </div>,
    );

    expect(screen.getByTestId("row").textContent).toBe("");
    expect(screen.queryByRole("link")).toBeNull();
  });
});
