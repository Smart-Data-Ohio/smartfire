import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { meFixture, messageFixture } from "../api/testing.ts";
import { MessageContent } from "../features/messages/message-content.tsx";
import type { ChatSounds } from "../gen/ChatSounds.ts";
import type { MessageSound } from "../gen/MessageSound.ts";
import type { SyncEvent } from "../gen/SyncEvent.ts";
import { emptyTimeline, initialState } from "../store/state.ts";
import { mutations, store } from "../store/store.ts";
import { notificationPreferencesFixture } from "../test/notification-fixtures.ts";
import { chatSoundsMuted, listenForChatSounds } from "./chat-sounds.ts";
import { emitSyncEvents } from "./signals.ts";

const SOUND: MessageSound = {
  name: "bell",
  url: "/assets/bell.mp3",
  presentation: { kind: "text", text: "🔔" },
};

const POLICY: ChatSounds = { muted: false, quietHours: null, timeZone: "UTC", quietWindows: [] };

const play = vi.fn<() => Promise<void>>();

let stop: (() => void) | null = null;

function arrive(
  id: number,
  {
    after = null,
    threadId = null,
    seq = id,
    liveAfter = 0,
  }: {
    readonly after?: number | null;
    readonly threadId?: number | null;
    readonly seq?: number;
    readonly liveAfter?: number;
  } = {},
) {
  const message = messageFixture(id, 12, { sound: SOUND, threadId });

  const timeline = {
    ...emptyTimeline,
    status: "ready",
    ids: [id],
    after,
  } satisfies typeof emptyTimeline;

  store.setState({
    messages: { ...store.getState().messages, [id]: message },
    ...(threadId === null
      ? { timelines: { 12: timeline } }
      : { threadTimelines: { [threadId]: timeline } }),
  });

  const event: SyncEvent = {
    seq,
    topic: threadId === null ? "room:12" : `thread:${threadId}`,
    type: "message.created",
    data: message,
  };

  emitSyncEvents([event], liveAfter);
}

beforeEach(() => {
  store.setState({ ...initialState, me: meFixture }, true);
  play.mockReset().mockResolvedValue(undefined);
  vi.spyOn(HTMLMediaElement.prototype, "play").mockImplementation(play);
});

afterEach(() => {
  stop?.();
  stop = null;
  vi.restoreAllMocks();
  vi.useRealTimers();
  store.setState(initialState, true);
});

describe("sound message", () => {
  it("renders classic text with a working manual play control, without autoplaying on mount", () => {
    render(<MessageContent message={messageFixture(1, 12, { sound: SOUND })} />);
    expect(screen.getByText("🔔").textContent).toContain("🔔");
    expect(play).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Play bell" }));
    expect(play).toHaveBeenCalledOnce();
    expect(play.mock.contexts[0]).toHaveProperty(
      "src",
      new URL("/assets/bell.mp3", document.baseURI).href,
    );
  });

  it("renders the classic image dimensions instead of the command text", () => {
    render(
      <MessageContent
        message={messageFixture(1, 12, {
          sound: {
            name: "56k",
            url: "/assets/56k.mp3",
            presentation: { kind: "image", url: "/assets/56k.webp", width: 79, height: 33 },
          },
        })}
      />,
    );
    expect(screen.getByRole("img", { name: "56k" }).getAttribute("width")).toBe("79");
    expect(screen.getByRole("img", { name: "56k" }).getAttribute("height")).toBe("33");
    expect(screen.queryByText("Message 1")).toBeNull();
  });

  it("applies quiet settings to manual playback too", () => {
    store.setState({ me: { ...meFixture, chatSounds: { ...POLICY, muted: true } } });
    render(<MessageContent message={messageFixture(1, 12, { sound: SOUND })} />);
    fireEvent.click(screen.getByRole("button", { name: "Play bell" }));
    expect(play).not.toHaveBeenCalled();
  });

  it("handles browser autoplay refusal without retrying history on the next gesture", async () => {
    play.mockRejectedValue(new DOMException("Gesture required", "NotAllowedError"));
    stop = listenForChatSounds(12, () => true);
    arrive(1);
    await Promise.resolve();
    fireEvent.pointerDown(document.body);
    expect(play).toHaveBeenCalledOnce();
  });
});

describe("automatic playback", () => {
  it.each(["2026-10-10T12:30:00Z", "2026-10-10T11:30:00Z"])(
    "keeps timed mutes until server expiry with browser time %s",
    async (browserTime) => {
      vi.useFakeTimers({ toFake: ["Date", "performance"] });
      vi.setSystemTime(new Date(browserTime));
      mutations.setNotificationPreferences(
        {
          ...notificationPreferencesFixture,
          roomMuteUntil: { "12": "2026-10-10T12:15:00Z" },
        },
        "2026-10-10T12:00:00Z",
      );
      stop = listenForChatSounds(12, () => true);
      arrive(1);
      expect(play).not.toHaveBeenCalled();
      await vi.advanceTimersByTimeAsync(15 * 60_000 - 1);
      arrive(2);
      expect(play).not.toHaveBeenCalled();
      vi.setSystemTime(new Date("2020-01-01T00:00:00Z"));
      await vi.advanceTimersByTimeAsync(1);
      arrive(3);
      expect(play).toHaveBeenCalledOnce();
    },
  );
  it("plays a live message once at the latest page", () => {
    stop = listenForChatSounds(12, () => true);
    arrive(1);
    arrive(1, { seq: 2 });
    expect(play).toHaveBeenCalledOnce();
  });

  it("never plays initial history, pagination, replay, or a remounted row", () => {
    arrive(1);
    stop = listenForChatSounds(12, () => true);
    arrive(1);
    arrive(2, { after: 99 });
    arrive(3, { liveAfter: 3 });
    stop();
    stop = listenForChatSounds(12, () => true);
    arrive(3, { seq: 4 });
    expect(play).not.toHaveBeenCalled();
  });

  it("leaves thread arrivals silent, as the classic thread controller does", () => {
    stop = listenForChatSounds(12, () => true);
    arrive(1, { threadId: 88 });
    arrive(2, { threadId: 88 });
    expect(play).not.toHaveBeenCalled();
    arrive(3);
    expect(play).toHaveBeenCalledOnce();
  });

  it("reads the current quiet setting on arrival and does not replay muted messages later", () => {
    stop = listenForChatSounds(12, () => true);
    store.setState({ me: { ...meFixture, chatSounds: { ...POLICY, muted: true } } });
    arrive(1);
    expect(play).not.toHaveBeenCalled();
    store.setState({ me: meFixture });
    arrive(1, { seq: 2 });
    arrive(2, { seq: 3 });
    expect(play).toHaveBeenCalledOnce();
  });
});

describe("classic quiet policy", () => {
  const at = new Date("2026-10-09T12:00:00Z");

  it("mutes manual or presence DND", () => {
    expect(chatSoundsMuted({ ...POLICY, muted: true }, at)).toBe(true);
  });

  it("checks the saved zone and inclusive-start/exclusive-end quiet-hour boundaries", () => {
    const policy = {
      ...POLICY,
      timeZone: "America/New_York",
      quietHours: { startMinute: 480, endMinute: 540 },
    };

    expect(chatSoundsMuted(policy, at)).toBe(true);
    expect(chatSoundsMuted(policy, new Date("2026-10-09T13:00:00Z"))).toBe(false);
  });

  it("wraps overnight, leaves empty windows audible, and ignores invalid zones", () => {
    const quietHours = { startMinute: 1320, endMinute: 420 };

    expect(chatSoundsMuted({ ...POLICY, quietHours }, new Date("2026-10-09T23:00:00Z"))).toBe(true);
    expect(chatSoundsMuted({ ...POLICY, quietHours }, new Date("2026-10-09T06:59:00Z"))).toBe(true);
    expect(chatSoundsMuted({ ...POLICY, quietHours }, new Date("2026-10-09T07:00:00Z"))).toBe(
      false,
    );
    expect(chatSoundsMuted({ ...POLICY, quietHours: { startMinute: 0, endMinute: 0 } }, at)).toBe(
      false,
    );
    expect(chatSoundsMuted({ ...POLICY, quietHours, timeZone: "Invalid" }, at)).toBe(false);
  });

  it("checks meeting and out-of-office windows on every play", () => {
    const second = at.getTime() / 1000;

    const policy: ChatSounds = {
      ...POLICY,
      quietWindows: [
        [second, second + 60],
        [second + 120, second + 180],
      ],
    };

    expect(chatSoundsMuted(policy, at)).toBe(true);
    expect(chatSoundsMuted(policy, new Date(at.getTime() + 60_000))).toBe(false);
    expect(chatSoundsMuted(policy, new Date(at.getTime() + 120_000))).toBe(true);
    expect(chatSoundsMuted(policy, new Date(at.getTime() + 180_000))).toBe(false);
  });
});
