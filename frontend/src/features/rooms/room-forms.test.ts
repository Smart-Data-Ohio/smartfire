import { describe, expect, it } from "vitest";
import type { RoomForm } from "../../gen/RoomForm.ts";
import { buildEmojiData } from "../../lib/emoji/data.ts";
import {
  availableChannels,
  channelOf,
  createBody,
  iconNameFor,
  isDirty,
  kindOf,
  privacyChoice,
  roomIconLook,
  updateBody,
} from "./room-forms.ts";

const EMOJI = buildEmojiData({
  groups: [
    {
      id: "activities",
      emoji: [
        ["🎉", "tada party", "party popper", "celebrate"],
        ["🚀", "rocket", "rocket", "launch"],
      ],
    },
  ],
});

function form(patch: Partial<RoomForm> = {}): RoomForm {
  return {
    type: "closed",
    roomId: 5,
    name: "Design",
    iconName: "tada",
    displayName: "Design",
    userIds: [1, 2],
    memberIds: [1, 2],
    candidateIds: [1, 2, 3],
    displayMemberIds: [],
    users: [],
    allowedTypes: ["open", "closed", "direct", "voice", "stage", "board"],
    conversionTypes: ["open", "closed"],
    canSubmit: true,
    canDelete: true,
    canLeave: false,
    groupCapable: false,
    defaultInvolvement: "mentions",
    stageRoles: [],
    ...patch,
  };
}

describe("channel kinds", () => {
  it("folds open and closed into one text channel with a private switch", () => {
    expect(kindOf("text", false)).toBe("open");
    expect(kindOf("text", true)).toBe("closed");
    expect(kindOf("voice", true)).toBe("voice");
    expect(channelOf("closed")).toBe("text");
    expect(channelOf("stage")).toBe("stage");
  });

  it("offers only what the workspace allows, in display order", () => {
    expect(availableChannels(["board", "closed", "direct"])).toEqual(["text", "board"]);
    expect(availableChannels(["direct"])).toEqual([]);
    expect(privacyChoice(["open", "closed"])).toBe("both");
    expect(privacyChoice(["closed", "voice"])).toBe("closed");
    expect(privacyChoice(["open"])).toBe("open");
  });
});

describe("createBody", () => {
  it("takes the kind's default for a blank name and sends members except for open rooms", () => {
    const draft = { name: "  ", iconName: null, userIds: [1, 4, 1] };

    expect(createBody("open", draft, "New room", "key-1")).toEqual({
      type: "open",
      clientRoomId: "key-1",
      name: "New room",
      iconName: null,
    });

    expect(
      createBody("voice", { ...draft, name: " Standup " }, "New voice channel", "key-2"),
    ).toEqual({
      type: "voice",
      clientRoomId: "key-2",
      name: "Standup",
      iconName: null,
      userIds: [1, 4],
    });
  });
});

describe("updateBody", () => {
  it("leaves unchanged attributes out, so a legacy icon stays writable", () => {
    expect(
      updateBody(form(), "closed", { name: "Design", iconName: "tada", userIds: [1] }),
    ).toEqual({ type: "closed", userIds: [1] });
  });

  it("sends changed attributes, null for a cleared name or icon", () => {
    expect(updateBody(form(), "open", { name: "", iconName: null, userIds: [1, 2] })).toEqual({
      type: "open",
      name: null,
      iconName: null,
    });

    expect(
      updateBody(form(), "closed", { name: "Design crit", iconName: "rocket", userIds: [1, 2] }),
    ).toEqual({ type: "closed", name: "Design crit", iconName: "rocket", userIds: [1, 2] });
  });
});

describe("isDirty", () => {
  it("notices a conversion, a rename, a new icon and a different member set", () => {
    const loaded = form();
    const same = { name: "Design", iconName: "tada", userIds: [2, 1] };

    expect(isDirty(loaded, "closed", same)).toBe(false);
    expect(isDirty(loaded, "open", same)).toBe(true);
    expect(isDirty(loaded, "closed", { ...same, name: "Design " })).toBe(false);
    expect(isDirty(loaded, "closed", { ...same, name: "Ops" })).toBe(true);
    expect(isDirty(loaded, "closed", { ...same, iconName: null })).toBe(true);
    expect(isDirty(loaded, "closed", { ...same, userIds: [1, 3] })).toBe(true);
  });

  it("ignores members for an open room, which everyone is in", () => {
    const loaded = form({ type: "open" });

    expect(isDirty(loaded, "open", { name: "Design", iconName: "tada", userIds: [] })).toBe(false);
  });
});

describe("icons", () => {
  it("turns a picked emoji into its first shortcode and an icon into its name", () => {
    expect(iconNameFor("🎉", EMOJI)).toBe("tada");
    expect(iconNameFor(":octocat:", EMOJI)).toBe("octocat");
    expect(iconNameFor("🦄", EMOJI)).toBeNull();
  });

  it("draws a workspace icon before the emoji of the same name", () => {
    const custom = [{ content: ":rocket:", title: "Rocket", imageUrl: "/icons/rocket" }];

    expect(roomIconLook("rocket", EMOJI, custom)).toEqual({
      kind: "image",
      url: "/icons/rocket",
      title: "Rocket",
    });

    expect(roomIconLook("party", EMOJI, custom)).toEqual({
      kind: "emoji",
      char: "🎉",
      title: "party popper",
    });

    expect(roomIconLook("gone", EMOJI, custom)).toEqual({ kind: "none" });
    expect(roomIconLook(null, EMOJI, custom)).toEqual({ kind: "none" });
  });
});
