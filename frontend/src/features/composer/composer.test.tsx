import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { emptyTimeline, initialState } from "../../store/state.ts";
import { store } from "../../store/store.ts";
import { composerActions } from "../../sync/composer-actions.ts";
import { actions } from "../../sync/runtime.ts";
import { clearCommandCache } from "./autocomplete/suggestions.ts";
import { Composer } from "./composer.tsx";
import { draftKey, readDraft } from "./draft.ts";

const ROOM = 12;

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
