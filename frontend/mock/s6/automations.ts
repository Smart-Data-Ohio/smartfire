import type { BoardAutomations } from "../../src/gen/BoardAutomations.ts";
import type { BoardSlaTimer } from "../../src/gen/BoardSlaTimer.ts";
import type { User } from "../../src/gen/User.ts";
import { forbidden, HttpError, notFound, ok } from "../http.ts";
import { field, intField, type Json, stringField } from "../json.ts";
import { firstId, route, type S2Context } from "../s2/context.ts";
import { VIEWER_ID, type World } from "../seed.ts";
import { ownerCandidates } from "./work.ts";

const SLA_ROWS = [
  { key: "planned", status: "planned", label: "Planned" },
  { key: "inProgress", status: "in_progress", label: "In progress" },
  { key: "blocked", status: "blocked", label: "Blocked" },
] as const;

const VALIDATION = "Validation";

function sentence(messages: readonly string[]): string {
  if (messages.length < 2) return messages.join("");

  const last = messages.at(-1);

  return messages.length === 2
    ? messages.join(" and ")
    : `${messages.slice(0, -1).join(", ")}, and ${last}`;
}

function eligibleAssignee(world: World, roomId: number, userId: number): boolean {
  const user = world.users.get(userId);

  return (
    user !== undefined &&
    (user.role !== "bot" || user.agent !== null) &&
    ownerCandidates(world, roomId).some((candidate) => candidate.userId === userId)
  );
}

/** The board's rules by tag, as `BoardTagAssignment::for_room` orders them (SQLite's binary order). */
function byTag<Rule extends { readonly tag: string }>(rules: readonly Rule[]): Rule[] {
  return [...rules].sort((a, b) => (a.tag < b.tag ? -1 : a.tag > b.tag ? 1 : 0));
}

/** The first rule by tag that matches applies, and an unavailable assignee's rule stops applying. */
export function autoAssignedBoardOwner(
  world: World,
  roomId: number,
  tags: readonly string[],
): User | null {
  const room = world.rooms.get(roomId);

  if (room?.room.kind !== "board") return null;

  const rule = byTag(room.boardAutomations?.tagRules ?? []).find((candidate) =>
    tags.includes(candidate.tag),
  );

  return rule === undefined || !eligibleAssignee(world, roomId, rule.assigneeId)
    ? null
    : (world.users.get(rule.assigneeId) ?? null);
}

type AutomationsContext = Pick<S2Context, "world" | "roomOr404" | "usersFor" | "publish">;

export function createBoardAutomations(ctx: AutomationsContext) {
  const board = (roomId: number) => {
    const room = ctx.roomOr404(roomId);
    const viewer = ctx.world().users.get(VIEWER_ID);

    if (
      room.room.kind !== "board" ||
      !room.memberIds.includes(VIEWER_ID) ||
      viewer?.status !== "active" ||
      viewer.role === "bot"
    )
      throw notFound("Board not found");

    if (room.room.creatorId !== VIEWER_ID && viewer.role !== "administrator")
      throw forbidden("Forbidden");

    room.boardAutomations ??= { tagRules: [], slaTimers: [], nextTagRuleId: 1 };

    return room.boardAutomations;
  };

  const settings = (roomId: number): BoardAutomations => {
    const state = board(roomId);
    const members = ctx.world().rooms.get(roomId)?.memberIds ?? [];

    const candidates = ctx
      .usersFor(members)
      .filter((user) => user.status === "active")
      .sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()) || a.id - b.id)
      .map((user) => user.id);

    return {
      roomId,
      tagRules: byTag(state.tagRules).map((rule) => ({ ...rule })),
      slaTimers: SLA_ROWS.flatMap(({ status }) =>
        state.slaTimers.filter((timer) => timer.status === status).map((timer) => ({ ...timer })),
      ),
      candidates,
      users: ctx.usersFor([...candidates, ...state.tagRules.map((rule) => rule.assigneeId)]),
    };
  };

  /** `board.automations.changed` on the room topic, so the board's other open panes refetch. */
  const changed = (roomId: number) =>
    ctx.publish([{ topic: `room:${roomId}`, type: "board.automations.changed", data: { roomId } }]);

  const addTagRule = (roomId: number, body: Json | undefined) => {
    const state = board(roomId);
    const tag = (stringField(body, "tag") ?? "").trim().toLowerCase();
    const assigneeId = intField(body, "assigneeId");
    const fields: Record<string, string[]> = {};
    const tagErrors: string[] = [];

    if (tag === "") tagErrors.push("can't be blank");

    if ([...tag].length > 30) tagErrors.push("is too long (maximum is 30 characters)");

    if (!/^[a-z0-9][a-z0-9-]*$/.test(tag)) tagErrors.push("is invalid");

    if (state.tagRules.some((rule) => rule.tag.toLowerCase() === tag))
      tagErrors.push("has already been taken");

    if (tagErrors.length > 0) fields.tag = tagErrors;

    if (assigneeId === null || !ctx.world().users.has(assigneeId)) {
      fields.assigneeId = ["must exist"];
    } else if (!eligibleAssignee(ctx.world(), roomId, assigneeId)) {
      fields.assigneeId = ["must be an active board member able to own posts"];
    }

    if (Object.keys(fields).length > 0) {
      const messages = [
        ...(fields.assigneeId ?? []).map((message) => `Assignee ${message}`),
        ...tagErrors.map((message) => `Tag ${message}`),
      ];

      throw new HttpError(422, { _tag: VALIDATION, message: sentence(messages), fields });
    }

    if (assigneeId === null) throw new Error("Validated assignee missing");
    state.tagRules.push({ id: state.nextTagRuleId++, tag, assigneeId });
    changed(roomId);

    return ok(settings(roomId), 201);
  };

  const removeTagRule = (roomId: number, ruleId: number) => {
    const state = board(roomId);
    const index = state.tagRules.findIndex((rule) => rule.id === ruleId);

    if (index === -1) throw notFound("Rule not found.");
    state.tagRules.splice(index, 1);
    changed(roomId);

    return ok(settings(roomId));
  };

  const saveSlaTimers = (roomId: number, body: Json | undefined) => {
    const state = board(roomId);
    const fields: Record<string, string[]> = {};
    const messages: string[] = [];
    const timers: BoardSlaTimer[] = [];

    for (const { key, status, label } of SLA_ROWS) {
      const row = field(body, key);

      if (row === undefined) {
        // A row left out leaves its status's timer as it stands.
        timers.push(...state.slaTimers.filter((timer) => timer.status === status));
        continue;
      }

      const nudge = intField(row, "nudgeAfterMinutes");
      const escalate = intField(row, "escalateAfterMinutes");

      if (field(row, "nudgeAfterMinutes") === null && field(row, "escalateAfterMinutes") === null)
        continue;
      const errors: string[] = [];

      for (const [name, value] of [
        ["Nudge after minutes", nudge],
        ["Escalate after minutes", escalate],
      ] as const) {
        if (value === null) errors.push(`${name} can't be blank`);
        else if (value <= 0) errors.push(`${name} must be greater than 0`);
        else if (value > 43200) errors.push(`${name} must be less than or equal to 43200`);
      }

      if (nudge !== null && escalate !== null && escalate <= nudge)
        errors.push("Escalate after minutes must be after the nudge threshold");

      if (errors.length > 0) {
        fields[key] = errors;
        messages.push(`${label}: ${sentence(errors)}`);
      } else if (nudge !== null && escalate !== null) {
        timers.push({ status, nudgeAfterMinutes: nudge, escalateAfterMinutes: escalate });
      }
    }

    if (messages.length > 0)
      throw new HttpError(422, { _tag: VALIDATION, message: sentence(messages), fields });

    const same = (a: readonly BoardSlaTimer[], b: readonly BoardSlaTimer[]) =>
      JSON.stringify(a) === JSON.stringify(b);

    const ordered = SLA_ROWS.flatMap(({ status }) =>
      timers.filter((timer) => timer.status === status),
    );

    if (
      !same(
        ordered,
        SLA_ROWS.flatMap(({ status }) =>
          state.slaTimers.filter((timer) => timer.status === status),
        ),
      )
    ) {
      state.slaTimers = ordered;
      changed(roomId);
    }

    return ok(settings(roomId));
  };

  return {
    routes: [
      route("GET", /^\/rooms\/(\d+)\/automations$/, (request) => ok(settings(firstId(request)))),
      route("POST", /^\/rooms\/(\d+)\/automations\/tag_rules$/, (request) =>
        addTagRule(firstId(request), request.body),
      ),
      route("DELETE", /^\/rooms\/(\d+)\/automations\/tag_rules\/(\d+)$/, (request) =>
        removeTagRule(firstId(request), request.ids[1] ?? 0),
      ),
      route("PUT", /^\/rooms\/(\d+)\/automations\/sla_timers$/, (request) =>
        saveSlaTimers(firstId(request), request.body),
      ),
    ],
  };
}
