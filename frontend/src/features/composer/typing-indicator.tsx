import type { State } from "../../store/state.ts";
import { useStore } from "../../store/store.ts";
import { AgentThinking } from "../../ui/agent-thinking.tsx";
import { UNKNOWN_NAME } from "../people/people.ts";

const NO_TYPISTS: readonly number[] = [];

/** `room:12`, or `thread:88` for a thread's composer. */
function conversation(roomId: number, threadId: number | null): string {
  return threadId === null ? `room:${roomId}` : `thread:${threadId}`;
}

function typistIds(
  state: State,
  roomId: number,
  threadId: number | null = null,
): readonly number[] {
  const entries = state.typing[conversation(roomId, threadId)];

  if (entries === undefined) {
    return NO_TYPISTS;
  }

  const ids = Object.keys(entries).map(Number);

  return ids.length === 0 ? NO_TYPISTS : ids;
}

/** Whether an agent is answering in this room: typing, or streaming one of the newest messages. */
export function useAgentReplying(roomId: number): boolean {
  return useStore((state) => {
    if (typistIds(state, roomId).some((id) => state.users[id]?.role === "bot")) {
      return true;
    }

    const ids = state.timelines[roomId]?.ids ?? [];

    return ids.slice(-5).some((id) => {
      const message = state.messages[id];

      return message?.streaming === true && state.users[message.creatorId]?.role === "bot";
    });
  });
}

/** "Ana is typing", "Ana and Ben are typing", "Several people are typing". */
export function typingSentence(names: readonly string[]): string {
  if (names.length === 1) {
    return `${names[0]} is typing`;
  }

  if (names.length === 2) {
    return `${names[0]} and ${names[1]} are typing`;
  }

  if (names.length === 3) {
    return `${names[0]}, ${names[1]} and ${names[2]} are typing`;
  }

  return "Several people are typing";
}

/**
 * The line under the composer. It always holds its height, so the composer never jumps; the text
 * fades in. Three dots pulse by opacity alone; an agent typing shows its thinking orb instead.
 */
export function TypingIndicator({
  roomId,
  threadId = null,
}: {
  readonly roomId: number;
  readonly threadId?: number | null;
}) {
  const sentence = useStore((state) => {
    const ids = typistIds(state, roomId, threadId);

    return ids.length === 0
      ? null
      : typingSentence(ids.map((id) => state.users[id]?.name ?? UNKNOWN_NAME));
  });

  const agent = useStore((state) =>
    typistIds(state, roomId, threadId).some((id) => state.users[id]?.role === "bot"),
  );

  return (
    <div className="typing" aria-live="polite">
      {sentence === null ? null : (
        <span key={sentence} className="typing-line enter-fade">
          {agent ? (
            <AgentThinking size={20} state="composing" label="Agent" />
          ) : (
            <span className="typing-dots" aria-hidden="true">
              <span />
              <span />
              <span />
            </span>
          )}
          <span>
            <strong>{sentence.replace(/ (is|are) typing$/, "")}</strong>
            {sentence.endsWith("are typing") ? " are typing…" : " is typing…"}
          </span>
        </span>
      )}
    </div>
  );
}
