import { describe, expect, it } from "@effect/vitest";
import { Schema } from "effect";
import { boardAutomations } from "../../test/board-fixtures.ts";
import {
  BoardAutomations,
  BoardSlaTimer,
  BoardSlaTimerInput,
  BoardTagRule,
  CreateBoardTagRule,
  UpdateBoardSlaTimers,
} from "./board-automations.ts";

const decode = Schema.decodeUnknownSync;

const off = { nudgeAfterMinutes: null, escalateAfterMinutes: null };

describe("board automation schemas", () => {
  it("decodes rules, timers, candidates, users and disabled settings", () => {
    const settings = decode(BoardAutomations)(boardAutomations());
    expect(settings.roomId).toBe(900);
    expect(settings.tagRules).toEqual([{ id: 1, tag: "bug", assigneeId: 7 }]);
    expect(settings.slaTimers[0]?.status).toBe("planned");
    expect(settings.candidates).toEqual([7]);
    expect(settings.users[0]?.id).toBe(7);
    expect(
      decode(BoardAutomations)({
        roomId: 900,
        tagRules: [],
        slaTimers: [],
        candidates: [],
        users: [],
      }),
    ).toEqual({
      roomId: 900,
      tagRules: [],
      slaTimers: [],
      candidates: [],
      users: [],
    });
  });

  it("preserves nulls and the camelCase SLA form keys", () => {
    expect(decode(CreateBoardTagRule)({ tag: " Bug ", assigneeId: null })).toEqual({
      tag: " Bug ",
      assigneeId: null,
    });
    expect(decode(CreateBoardTagRule)({ tag: "design", assigneeId: 7 }).assigneeId).toBe(7);
    expect(decode(BoardSlaTimerInput)(off)).toEqual(off);

    const input = {
      planned: off,
      inProgress: { nudgeAfterMinutes: 60, escalateAfterMinutes: 120 },
      blocked: off,
    };

    expect(decode(UpdateBoardSlaTimers)(input)).toEqual(input);
    expect(decode(UpdateBoardSlaTimers)({ blocked: off })).toEqual({ blocked: off });
    expect(decode(BoardSlaTimerInput)({ ...off, nudgeAfterMinutes: 60 })).toEqual({
      nudgeAfterMinutes: 60,
      escalateAfterMinutes: null,
    });
  });

  it("rejects missing fields, wrong ids, unknown statuses and fractional minutes", () => {
    expect(() => decode(BoardTagRule)({ id: 1, tag: "bug", assigneeId: "7" })).toThrow();
    expect(() => decode(BoardTagRule)({ id: 1.5, tag: "bug", assigneeId: 7 })).toThrow();
    expect(() =>
      decode(BoardSlaTimer)({ status: "unknown", nudgeAfterMinutes: 1, escalateAfterMinutes: 2 }),
    ).toThrow();
    expect(() =>
      decode(BoardSlaTimer)({ status: "planned", nudgeAfterMinutes: 1.5, escalateAfterMinutes: 2 }),
    ).toThrow();
    expect(() =>
      decode(BoardSlaTimerInput)({ nudgeAfterMinutes: "1", escalateAfterMinutes: null }),
    ).toThrow();
    expect(() => decode(CreateBoardTagRule)({ tag: "bug" })).toThrow();
    expect(() =>
      decode(UpdateBoardSlaTimers)({ planned: off, inProgress: null, blocked: off }),
    ).toThrow();
    expect(() => decode(BoardAutomations)({ ...boardAutomations(), candidates: ["7"] })).toThrow();
    const { users: _users, ...missing } = boardAutomations();
    expect(() => decode(BoardAutomations)(missing)).toThrow();
  });
});
