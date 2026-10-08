import { afterEach, describe, expect, it } from "vitest";
import { userFixture } from "../api/testing.ts";
import type { AgentDirectoryRow } from "../gen/AgentDirectoryRow.ts";
import type { AgentProfile } from "../gen/AgentProfile.ts";
import type { AgentStatus } from "../gen/AgentStatus.ts";
import type { User } from "../gen/User.ts";
import { nextObservation, observeResponse } from "../lib/request-observation.ts";
import {
  captureAgentRead,
  landDirectory,
  landProfile,
  setDirectoryLoading,
  setProfileLoading,
} from "./agents.ts";
import { initialState } from "./state.ts";
import { mutations, store } from "./store.ts";

const row: AgentDirectoryRow = {
  agentId: 40,
  userId: 40,
  ownerId: 1,
  kind: "workspace",
  status: "idle",
  statusNote: null,
  suspended: false,
  statusChangedAt: null,
  lastSeenAt: null,
  createdAt: "2026-10-07T10:00:00.000Z",
  updatedAt: "2026-10-07T10:00:00.000000Z",
};

function profile(agent: AgentDirectoryRow): AgentProfile {
  return {
    agent,
    provider: null,
    runtime: null,
    description: null,
    rooms: [],
    hiddenRoomCount: 0,
    grants: null,
    management: null,
    users: [userFixture(40)],
  };
}

describe("agent fields observed outside the row revision", () => {
  it("keeps a fresh profile's last-seen time and ownership against a delayed tied directory", () => {
    const loading = setDirectoryLoading(initialState);
    const read = captureAgentRead(loading);
    const opened = setProfileLoading(loading, 40);
    const fresh = { ...row, ownerId: 2, lastSeenAt: "2026-10-07T10:01:00.000Z" };
    const loaded = landProfile(opened, profile(fresh), 1, captureAgentRead(opened));
    const late = landDirectory(loaded, { agents: [row], users: [] }, 1, read);

    expect(late.agents.rows[40]).toEqual(fresh);
    expect(late.agents.profiles[40]?.profile?.agent).toEqual(fresh);
  });

  it("an unchanged observation protects fields while a later status revision still lands", () => {
    const loading = setDirectoryLoading(initialState);
    const loaded = landDirectory(loading, { agents: [row], users: [] }, 1);
    const read = captureAgentRead(loaded);

    const refreshed = landDirectory(
      loaded,
      { agents: [row], users: [] },
      1,
      captureAgentRead(loaded),
    );

    const incoming = {
      ...row,
      ownerId: 2,
      lastSeenAt: "2026-10-07T10:01:00.000Z",
      status: "working",
      updatedAt: "2026-10-07T10:00:00.000001Z",
    } satisfies AgentDirectoryRow;

    const late = landDirectory(refreshed, { agents: [incoming], users: [] }, 1, read);

    expect(late.agents.rows[40]).toEqual({
      ...row,
      status: "working",
      updatedAt: incoming.updatedAt,
    });
  });

  it("lands the newer of overlapping reads even when the older response arrives first", () => {
    const loading = setDirectoryLoading(initialState);
    const older = captureAgentRead(loading);
    const newer = captureAgentRead(loading);
    const first = landDirectory(loading, { agents: [row], users: [] }, 1, older);
    const fresh = { ...row, ownerId: 2, lastSeenAt: "2026-10-07T10:01:00.000Z" };
    const second = landDirectory(first, { agents: [fresh], users: [] }, 1, newer);

    expect(second.agents.rows[40]).toEqual(fresh);
  });
});

describe("agent badge observations with cached revision-bearing status", () => {
  afterEach(() => mutations.reset());

  const bot = (status: AgentStatus, suspended: boolean): User => ({
    ...userFixture(40),
    role: "bot",
    agent: { agentId: 40, kind: "workspace", status, suspended },
  });

  function directory(at: number) {
    mutations.setAgentDirectoryLoading();
    mutations.landAgentDirectory(
      { agents: [row], users: [] },
      store.getState().agents.directory.generation,
      { at },
    );
  }

  it("allows a fresh User badge to supersede cached status and rejects a delayed User copy", () => {
    directory(nextObservation());
    const delayed = bot("idle", false);
    observeResponse(delayed, nextObservation());
    const fresh = bot("working", true);
    observeResponse(fresh, nextObservation());

    mutations.mergeUsers([fresh]);
    expect(store.getState().users[40]?.agent).toEqual(fresh.agent);
    mutations.mergeUsers([delayed]);
    expect(store.getState().users[40]?.agent).toEqual(fresh.agent);
  });

  it("protects a newer status event's badge from an earlier User request", () => {
    directory(nextObservation());
    const delayed = bot("idle", false);
    observeResponse(delayed, nextObservation());
    mutations.mergeUsers([bot("idle", false)]);
    mutations.applyEvents(
      [
        {
          seq: 1,
          topic: "user:40",
          type: "agent.status",
          data: {
            ...row,
            status: "working",
            suspended: true,
            updatedAt: "2026-10-07T10:00:00.000001Z",
            workingPresence: null,
            workingPresenceExpiresAt: null,
          },
        },
      ],
      0,
    );

    mutations.mergeUsers([delayed]);
    expect(store.getState().users[40]?.agent).toEqual(bot("working", true).agent);
  });

  it("advances an identical badge observation without reverting the avatar observation", () => {
    directory(nextObservation());
    mutations.mergeUsers([bot("idle", false)]);
    const delayed = bot("working", true);
    observeResponse(delayed, nextObservation());
    const avatar = { ...bot("idle", false), hasAvatar: true };
    observeResponse(avatar, nextObservation());
    mutations.mergeUsers([avatar]);
    directory(nextObservation());

    mutations.mergeUsers([delayed]);
    expect(store.getState().users[40]).toMatchObject({
      hasAvatar: true,
      agent: bot("idle", false).agent,
    });
  });
});
