import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { sidebarFixture, sidebarRowFixture } from "../../api/testing.ts";
import type { AnnouncementFollow } from "../../gen/AnnouncementFollow.ts";
import type { SidebarRow } from "../../gen/SidebarRow.ts";
import { mutations, sidebarRowClock } from "../../store/store.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { AnnouncementSettings } from "./announcement-settings.tsx";

function row(roomId: number, name: string, announcement: boolean): SidebarRow {
  const base = sidebarRowFixture(roomId, name);

  return { ...base, room: { ...base.room, announcement } };
}

function follow(sourceRoomId: number, targetRoomId: number): AnnouncementFollow {
  return { sourceRoomId, targetRoomId, creatorId: 7, createdAt: "2026-10-11T00:00:00.000Z" };
}

const ENGINEERING = 3;

const NEWS = 5;

const RELEASES = 6;

beforeEach(() => {
  mutations.reset();
  mutations.loadSidebar(
    sidebarFixture([
      row(ENGINEERING, "engineering", false),
      row(NEWS, "news", true),
      row(RELEASES, "releases", true),
    ]),
    sidebarRowClock(),
  );
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("Announcement settings", () => {
  it("lists what the room follows, follows another announcement room and unfollows", async () => {
    vi.spyOn(actions.announcements, "follows")
      .mockResolvedValueOnce({ followers: [], following: [follow(NEWS, ENGINEERING)] })
      .mockResolvedValueOnce({
        followers: [],
        following: [follow(NEWS, ENGINEERING), follow(RELEASES, ENGINEERING)],
      });

    const followAction = vi
      .spyOn(actions.announcements, "follow")
      .mockResolvedValue({ followers: [follow(RELEASES, ENGINEERING)], following: [] });

    const unfollow = vi.spyOn(actions.announcements, "unfollow").mockResolvedValue(undefined);
    const user = userEvent.setup();

    render(<AnnouncementSettings roomId={ENGINEERING} kind="open" manage />);

    const following = await screen.findByRole("region", { name: "Following" });

    expect(within(following).getByText("news")).toBeTruthy();
    // Only announcement rooms the room doesn't follow yet are offered.
    const picker = within(following).getByLabelText("Announcement channel to follow");

    expect(
      within(picker)
        .getAllByRole("option")
        .map((option) => option.textContent),
    ).toEqual(["releases"]);

    await user.click(within(following).getByRole("button", { name: "Follow" }));

    // The source room's URL takes this room as the target.
    expect(followAction).toHaveBeenCalledWith(RELEASES, ENGINEERING);
    expect(await within(following).findByText("releases")).toBeTruthy();
    expect(
      await within(following).findByText(
        "This channel follows every announcement channel you're in.",
      ),
    ).toBeTruthy();

    await user.click(within(following).getByRole("button", { name: "Unfollow news" }));

    expect(unfollow).toHaveBeenCalledWith(NEWS, ENGINEERING);
    await waitFor(() =>
      expect(
        within(following)
          .getAllByRole("listitem")
          .map((item) => item.querySelector(".room-follow-name")?.textContent),
      ).toEqual(["releases"]),
    );
    // It can be followed again.
    expect(
      within(within(following).getByLabelText("Announcement channel to follow"))
        .getAllByRole("option")
        .map((option) => option.textContent),
    ).toEqual(["news"]);
  });

  it("shows the toggle only to managers and reads the list without controls otherwise", async () => {
    vi.spyOn(actions.announcements, "follows").mockResolvedValue({
      followers: [],
      following: [follow(NEWS, ENGINEERING)],
    });

    render(<AnnouncementSettings roomId={ENGINEERING} kind="open" manage={false} />);

    const following = await screen.findByRole("region", { name: "Following" });

    expect(within(following).getByText("news")).toBeTruthy();
    expect(screen.queryByRole("switch")).toBeNull();
    expect(screen.queryByRole("button", { name: /Unfollow/ })).toBeNull();
    expect(screen.queryByLabelText("Announcement channel to follow")).toBeNull();
    expect(screen.getByText(/Only the person who made this room and administrators/)).toBeTruthy();
  });

  it("flips the flag through the API and lists followers of an announcement room", async () => {
    vi.spyOn(actions.announcements, "follows").mockResolvedValue({
      followers: [follow(NEWS, ENGINEERING)],
      following: [],
    });

    const set = vi
      .spyOn(actions.announcements, "set")
      .mockImplementation((roomId, value) =>
        Promise.resolve({ ...row(roomId, "news", value).room }),
      );

    const user = userEvent.setup();

    render(<AnnouncementSettings roomId={NEWS} kind="open" manage />);

    const toggle = screen.getByRole("switch", { name: /Announcement channel/ });

    expect(toggle.getAttribute("aria-checked")).toBe("true");
    expect(await screen.findByRole("region", { name: "Followed by" })).toBeTruthy();

    await user.click(toggle);

    expect(set).toHaveBeenCalledWith(NEWS, false);
    await waitFor(() => expect(toggle.getAttribute("aria-checked")).toBe("false"));
    expect(screen.queryByRole("region", { name: "Followed by" })).toBeNull();
  });

  it("shows the server's refusal and leaves a board without follows", async () => {
    vi.spyOn(actions.announcements, "follows").mockResolvedValue({ followers: [], following: [] });
    vi.spyOn(actions.announcements, "follow").mockRejectedValue(
      new ActionError("Validation", "Target room would create an announcement cycle", {
        targetRoomId: ["would create an announcement cycle"],
      }),
    );

    const user = userEvent.setup();
    const { unmount } = render(<AnnouncementSettings roomId={ENGINEERING} kind="open" manage />);

    await user.click(await screen.findByRole("button", { name: "Follow" }));

    expect((await screen.findByRole("alert")).textContent).toContain(
      "would create an announcement cycle",
    );

    unmount();
    render(<AnnouncementSettings roomId={ENGINEERING} kind="board" manage />);

    expect(screen.getByRole("switch", { name: /Announcement channel/ })).toBeTruthy();
    expect(screen.queryByRole("region", { name: "Following" })).toBeNull();
  });
});
