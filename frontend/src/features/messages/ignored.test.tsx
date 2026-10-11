import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { messageFixture, userFixture } from "../../api/testing.ts";
import type { MessageDTO } from "../../store/model.ts";
import { emptyTimeline } from "../../store/state.ts";
import { mutations, store, useStore } from "../../store/store.ts";
import { timelineItems } from "../room/timeline-items.ts";
import { threadDetailFixture } from "../work/test-fixtures.ts";
import { IgnoredGate, IgnoredNotice, ignoredLabel, useIgnoredReveal } from "./message-content.tsx";

const ROOM = 4;

afterEach(() => {
  mutations.reset();
});

/** A timeline's rows as text, with the ignored lines the real timeline renders. */
function Rows({
  ids,
  focus = null,
  navigation = "first",
}: {
  readonly ids: readonly number[];
  readonly focus?: number | null;
  readonly navigation?: string;
}) {
  const messages = useStore((state) => state.messages);
  const ignores = useStore((state) => state.ignores);
  const reveal = useIgnoredReveal(focus, navigation);

  const items = timelineItems({
    timeline: { ...emptyTimeline, status: "ready", ids },
    messages,
    pending: [],
    now: Date.now(),
    ignores,
    revealed: reveal.revealed,
    focusId: reveal.focusId,
  });

  return (
    <div>
      {items.map((item) => {
        switch (item.kind) {
          case "ignored":
            return (
              <IgnoredNotice
                key={item.key}
                count={item.ids.length}
                revealed={item.revealed}
                onToggle={() => reveal.toggle(item.ids, !item.revealed)}
              />
            );
          case "message":
            return <p key={item.key}>{item.message.markdownSource}</p>;
          default:
            return null;
        }
      })}
    </div>
  );
}

function install(messages: readonly MessageDTO[]) {
  store.setState({
    messages: Object.fromEntries(messages.map((message) => [message.id, message])),
  });
}

describe("ignored messages", () => {
  it("words the line for one message and for several", () => {
    expect(ignoredLabel(1)).toBe("1 message from someone you ignore");
    expect(ignoredLabel(3)).toBe("3 messages from someone you ignore");
  });

  it("collapses consecutive ignored messages into one line and shows them on request", () => {
    install([
      messageFixture(1, ROOM),
      messageFixture(2, ROOM, { creatorId: 9, ignored: true }),
      messageFixture(3, ROOM, { creatorId: 9, ignored: true }),
      messageFixture(4, ROOM),
    ]);
    render(<Rows ids={[1, 2, 3, 4]} />);

    expect(screen.getByText("2 messages from someone you ignore")).toBeTruthy();
    expect(screen.queryByText("Message 2")).toBeNull();
    expect(screen.queryByText("Message 3")).toBeNull();
    expect(screen.getByText("Message 1")).toBeTruthy();
    expect(screen.getByText("Message 4")).toBeTruthy();

    const show = screen.getByRole("button", { name: "Show" });

    expect(show.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(show);

    expect(screen.getByText("Message 2")).toBeTruthy();
    expect(screen.getByText("Message 3")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Hide" }).getAttribute("aria-expanded")).toBe("true");

    fireEvent.click(screen.getByRole("button", { name: "Hide" }));
    expect(screen.queryByText("Message 2")).toBeNull();
  });

  it("keeps a reveal on this screen only: a fresh one starts collapsed", () => {
    install([messageFixture(2, ROOM, { creatorId: 9, ignored: true })]);

    const first = render(<Rows ids={[2]} />);

    fireEvent.click(screen.getByRole("button", { name: "Show" }));
    expect(screen.getByText("Message 2")).toBeTruthy();
    first.unmount();

    render(<Rows ids={[2]} />);
    expect(screen.queryByText("Message 2")).toBeNull();
    expect(screen.getByText("1 message from someone you ignore")).toBeTruthy();
  });

  it("opens the run a permalink lands in, until the reader hides it", () => {
    install([messageFixture(2, ROOM, { creatorId: 9, ignored: true })]);
    render(<Rows ids={[2]} focus={2} />);

    expect(screen.getByText("Message 2")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Hide" }));
    expect(screen.queryByText("Message 2")).toBeNull();
  });

  it("collapses and reopens held messages as an ignore change lands, with no refetch", () => {
    install([messageFixture(1, ROOM, { creatorId: 9 }), messageFixture(2, ROOM, { creatorId: 9 })]);
    render(<Rows ids={[1, 2]} />);
    expect(screen.getByText("Message 1")).toBeTruthy();

    act(() => mutations.applyBlockState({ userId: 9, ignoredByMe: true, revision: 5 }));
    expect(screen.getByText("2 messages from someone you ignore")).toBeTruthy();
    expect(screen.queryByText("Message 1")).toBeNull();

    // The sync echo of the same write changes nothing.
    act(() => mutations.applyBlockState({ userId: 9, ignoredByMe: false, revision: 5 }));
    expect(screen.queryByText("Message 1")).toBeNull();

    act(() => mutations.applyBlockState({ userId: 9, ignoredByMe: false, revision: 6 }));
    expect(screen.getByText("Message 1")).toBeTruthy();
    expect(screen.queryByText(/from someone you ignore/)).toBeNull();
  });

  it("gates a lone message (a thread's parent) behind the same line", () => {
    const ignored = messageFixture(2, ROOM, { creatorId: 9, ignored: true });

    const { rerender } = render(
      <IgnoredGate message={ignored}>
        <p>Parent body</p>
      </IgnoredGate>,
    );

    expect(screen.queryByText("Parent body")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Show" }));
    expect(screen.getByText("Parent body")).toBeTruthy();

    rerender(
      <IgnoredGate message={{ ...ignored, ignored: false }}>
        <p>Parent body</p>
      </IgnoredGate>,
    );
    expect(screen.queryByRole("button", { name: /Show|Hide/ })).toBeNull();
    expect(screen.getByText("Parent body")).toBeTruthy();
  });

  it("shows a permalink's target again on every navigation to it, even after Hide", () => {
    install([
      messageFixture(2, ROOM, { creatorId: 9, ignored: true }),
      messageFixture(3, ROOM),
      messageFixture(4, ROOM, { creatorId: 9, ignored: true }),
    ]);

    const { rerender } = render(<Rows ids={[2, 3, 4]} focus={2} navigation="a" />);

    expect(screen.getByText("Message 2")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Hide" }));
    expect(screen.queryByText("Message 2")).toBeNull();

    rerender(<Rows ids={[2, 3, 4]} focus={4} navigation="b" />);
    expect(screen.getByText("Message 4")).toBeTruthy();
    expect(screen.queryByText("Message 2")).toBeNull();

    rerender(<Rows ids={[2, 3, 4]} focus={2} navigation="c" />);
    expect(screen.getByText("Message 2")).toBeTruthy();
  });

  it("keeps a thread's parent open when a late detail still calls its author ignored", () => {
    const parent = messageFixture(2, ROOM, { creatorId: 9, threadId: 50 });

    install([parent]);
    mutations.setBlocks({
      revision: 4,
      people: [{ user: userFixture(9), ignoredByMe: true, createdAt: "2026-10-01T12:00:00Z" }],
    });
    mutations.applyBlockState({ userId: 9, ignoredByMe: false, revision: 5 });

    function Parent() {
      const message = useStore((state) => state.messages[2]);

      return message === undefined ? null : (
        <IgnoredGate message={message}>
          <p>Parent body</p>
        </IgnoredGate>
      );
    }

    render(
      <>
        <Parent />
        <Rows ids={[2]} />
      </>,
    );

    // The detail was requested before the unignore and answers after it, newer than what's held.
    act(() =>
      mutations.loadThreadDetail({
        ...threadDetailFixture(50, null, null),
        parentMessage: { ...parent, ignored: true, updatedAt: "2026-10-11T12:00:00.000Z" },
      }),
    );

    expect(store.getState().messages[2]?.ignored).toBe(true);
    expect(screen.getByText("Parent body")).toBeTruthy();
    expect(screen.getByText("Message 2")).toBeTruthy();
    expect(screen.queryByText(/from someone you ignore/)).toBeNull();
  });
});
