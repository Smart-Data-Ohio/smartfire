import { describe, expect, it } from "vitest";
import { messageFixture, pageFixture, userFixture } from "../api/testing.ts";
import type { AgentDirectoryRow } from "../gen/AgentDirectoryRow.ts";
import type { AgentStatusChanged } from "../gen/AgentStatusChanged.ts";
import type { AgentStep } from "../gen/AgentStep.ts";
import type { User } from "../gen/User.ts";
import {
  directoryGeneration,
  landDirectory,
  landProfile,
  mergeMessageCopies,
  mergeSteps,
  nextWorkingExpiry,
  profileOf,
  roomAgentIds,
  setDirectoryFailed,
  setDirectoryLoading,
  setProfileFailed,
  workingPresenceAt,
} from "./agents.ts";
import type { SyncEvent } from "./model.ts";
import { applyEvents, applyPage, receiveMessage, updateMessage } from "./reducers.ts";
import { initialState, type State } from "./state.ts";

const NOW = Date.UTC(2026, 9, 6, 16, 30, 0);

const ROOM = 12;

function at(second: number): string {
  return new Date(NOW + second * 1000).toISOString();
}

function bot(id: number, name: string, change: Partial<User> = {}): User {
  return {
    ...userFixture(id, name),
    role: "bot",
    agent: { agentId: id, kind: "workspace", status: "idle", suspended: false },
    ...change,
  };
}

function row(agentId: number, change: Partial<AgentDirectoryRow> = {}): AgentDirectoryRow {
  return {
    agentId,
    userId: agentId,
    kind: "workspace",
    ownerId: 1,
    status: "idle",
    statusNote: null,
    suspended: false,
    createdAt: at(-1000),
    statusChangedAt: null,
    lastSeenAt: null,
    ...change,
  };
}

function step(id: number, updated: number, change: Partial<AgentStep> = {}): AgentStep {
  return {
    id,
    messageId: 100,
    threadId: null,
    name: `Step ${id}`,
    status: "done",
    inputSummary: null,
    outputSummary: null,
    durationMs: 400,
    position: id,
    createdAt: at(0),
    updatedAt: at(updated),
    ...change,
  };
}

function status(change: Partial<AgentStatusChanged> = {}): AgentStatusChanged {
  return {
    agentId: 40,
    userId: 40,
    status: "working",
    statusNote: null,
    statusChangedAt: at(0),
    suspended: false,
    workingPresence: "Reading the logs",
    workingPresenceExpiresAt: at(300),
    ...change,
  };
}

/** The directory loaded with Bea (40) and Ada (41), both idle. */
function withDirectory(): State {
  const loading = setDirectoryLoading(initialState);

  return landDirectory(
    loading,
    { agents: [row(41), row(40)], users: [bot(40, "Bea"), bot(41, "Ada"), userFixture(1)] },
    directoryGeneration(loading),
  );
}

function event(payload: SyncEvent): State {
  return applyEvents(withDirectory(), [payload], NOW);
}

describe("the agent directory and profiles", () => {
  it("lands a load only in the generation it started in", () => {
    const first = setDirectoryLoading(initialState);
    const stale = directoryGeneration(first);
    const second = setDirectoryLoading(first);
    const page = { agents: [row(40)], users: [bot(40, "Bea")] };

    expect(landDirectory(second, page, stale)).toBe(second);
    expect(landDirectory(second, page, directoryGeneration(second)).agents.directory).toMatchObject(
      {
        ids: [40],
        status: "ready",
      },
    );
  });

  it("keeps the rows shown when a reload fails", () => {
    const loaded = withDirectory();
    const reloading = setDirectoryLoading(loaded);
    const failed = setDirectoryFailed(reloading, "Offline", directoryGeneration(reloading));

    expect(failed.agents.directory).toMatchObject({
      ids: [41, 40],
      status: "ready",
      error: "Offline",
    });
  });

  it("marks a 404 profile missing, and keeps a shown one on another failure", () => {
    const missing = setProfileFailed(initialState, 40, "Agent not found", true);

    expect(profileOf(missing, 40)).toMatchObject({ status: "error", missing: true });

    const profile = {
      agent: row(40),
      provider: null,
      runtime: null,
      description: null,
      rooms: [],
      hiddenRoomCount: 0,
      grants: null,
      management: null,
      users: [bot(40, "Bea")],
    };

    const shown = setProfileFailed(landProfile(initialState, profile), 40, "Offline", false);

    expect(profileOf(shown, 40)).toMatchObject({ status: "ready", error: "Offline" });
    expect(shown.users[40]?.name).toBe("Bea");
  });
});

describe("agent.status", () => {
  it("updates the badge, the row and the working presence, and re-sorts on suspension", () => {
    const next = event({
      seq: 1,
      topic: "user:1",
      type: "agent.status",
      data: status({ agentId: 41, userId: 41, suspended: true }),
    });

    expect(next.users[41]?.agent).toMatchObject({ status: "working", suspended: true });
    expect(next.agents.rows[41]).toMatchObject({ status: "working", suspended: true });
    // Suspended agents sort after active ones.
    expect(next.agents.directory.ids).toEqual([40, 41]);
    expect(workingPresenceAt(next, 41, NOW)).toBe("Reading the logs");
  });

  it("lets the working presence lapse at expiresAt, with no event", () => {
    const next = event({ seq: 1, topic: "user:1", type: "agent.status", data: status() });

    expect(nextWorkingExpiry(next, NOW)).toBe(NOW + 300_000);
    expect(workingPresenceAt(next, 40, NOW + 299_000)).toBe("Reading the logs");
    expect(workingPresenceAt(next, 40, NOW + 300_000)).toBeNull();
    expect(nextWorkingExpiry(next, NOW + 300_000)).toBeNull();
  });

  it("clears the presence when the next change carries none", () => {
    const working = event({ seq: 1, topic: "user:1", type: "agent.status", data: status() });

    const idle = applyEvents(
      working,
      [
        {
          seq: 2,
          topic: "user:1",
          type: "agent.status",
          data: status({ status: "idle", workingPresence: null, workingPresenceExpiresAt: null }),
        },
      ],
      NOW,
    );

    expect(workingPresenceAt(idle, 40, NOW)).toBeNull();
    expect(idle.users[40]?.agent?.status).toBe("idle");
  });
});

describe("agent steps", () => {
  it("merges each step by updatedAt, a tie going to the later arrival", () => {
    const held = [step(1, 5, { status: "running" }), step(2, 5, { status: "pending" })];
    const incoming = [step(1, 3, { status: "pending" }), step(2, 5, { status: "running" })];

    expect(mergeSteps(held, incoming).map((each) => each.status)).toEqual(["running", "running"]);
  });

  it("answers the held steps when nothing changed, in (position, id) order otherwise", () => {
    const held = [step(1, 5)];

    expect(mergeSteps(held, [step(1, 5)])).toBe(held);
    expect(mergeSteps(held, [step(3, 1, { position: 0 })]).map((each) => each.id)).toEqual([3, 1]);
  });

  it("keeps the newer steps when a copy of the message with older ones lands", () => {
    const held = messageFixture(100, ROOM, { steps: [step(1, 9, { status: "done" })] });

    const incoming = messageFixture(100, ROOM, {
      markdownSource: "Edited",
      updatedAt: at(50),
      steps: [step(1, 2, { status: "running" })],
    });

    const merged = mergeMessageCopies(held, incoming, "held");

    expect(merged.markdownSource).toBe("Edited");
    expect(merged.steps[0]?.status).toBe("done");
  });

  it("applies agent.steps to a held message, and leaves a work thread's to the work view", () => {
    const state = applyPage(
      initialState,
      ROOM,
      pageFixture([messageFixture(100, ROOM)]),
      "replace",
    );

    const stepped = applyEvents(
      state,
      [
        {
          seq: 1,
          topic: `room:${ROOM}`,
          type: "agent.steps",
          data: {
            roomId: ROOM,
            messageId: 100,
            threadId: null,
            steps: [step(1, 1, { status: "running" })],
          },
        },
      ],
      NOW,
    );

    expect(stepped.messages[100]?.steps.map((each) => each.status)).toEqual(["running"]);

    const thread = applyEvents(
      stepped,
      [
        {
          seq: 2,
          topic: `room:${ROOM}`,
          type: "agent.steps",
          data: { roomId: ROOM, messageId: null, threadId: 7, steps: [step(2, 1)] },
        },
      ],
      NOW,
    );

    expect(thread).toBe(stepped);
  });

  it("keeps steps an update or a re-received copy doesn't know yet", () => {
    const state = applyPage(
      initialState,
      ROOM,
      pageFixture([messageFixture(100, ROOM, { steps: [step(1, 9)] })]),
      "replace",
    );

    const updated = updateMessage(
      state,
      messageFixture(100, ROOM, { updatedAt: at(60), steps: [] }),
    );

    expect(updated.messages[100]?.steps.map((each) => each.id)).toEqual([1]);

    const received = receiveMessage(
      updated,
      messageFixture(100, ROOM, { updatedAt: at(60), steps: [] }),
    );

    expect(received.messages[100]?.steps.map((each) => each.id)).toEqual([1]);
  });
});

describe("a room's agents", () => {
  it("counts the agents among the members shown and the newest authors", () => {
    const state = applyPage(
      { ...withDirectory(), users: { ...withDirectory().users, 7: userFixture(7) } },
      ROOM,
      pageFixture([messageFixture(100, ROOM, { creatorId: 41 }), messageFixture(101, ROOM)]),
      "replace",
    );

    expect(roomAgentIds(state, ROOM)).toEqual([41]);
  });
});
