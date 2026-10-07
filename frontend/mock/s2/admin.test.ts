import { Schema } from "effect";
import { describe, expect, it } from "vitest";
import {
  AuditLogPage as AuditLogPageSchema,
  CustomStyles as CustomStylesSchema,
  IntegrationsHealth as IntegrationsHealthSchema,
  PeoplePage as PeoplePageSchema,
  WorkspaceIconList as WorkspaceIconListSchema,
  Workspace as WorkspaceSchema,
} from "../../src/api/schema/admin.ts";
import type { AuditLogPage } from "../../src/gen/AuditLogPage.ts";
import type { CustomStyles } from "../../src/gen/CustomStyles.ts";
import type { IntegrationsHealth } from "../../src/gen/IntegrationsHealth.ts";
import type { PeoplePage } from "../../src/gen/PeoplePage.ts";
import type { PersonChange } from "../../src/gen/PersonChange.ts";
import type { Workspace } from "../../src/gen/Workspace.ts";
import type { WorkspaceIconList } from "../../src/gen/WorkspaceIconList.ts";
import { field, type Json } from "../json.ts";
import { BOT_ID, DEACTIVATED_ID, USER_IDS, VIEWER_ID } from "../seed.ts";
import { errorOf, expectStatus, get, harness } from "./testing.ts";

describe("the mock's admin pages", () => {
  it("answer the contract's shapes", async () => {
    const { server } = harness();

    Schema.decodeUnknownSync(WorkspaceSchema)(
      await get<Workspace>(server, "/api/v1/admin/workspace"),
    );
    Schema.decodeUnknownSync(PeoplePageSchema)(
      await get<PeoplePage>(server, "/api/v1/admin/people"),
    );
    Schema.decodeUnknownSync(CustomStylesSchema)(
      await get<CustomStyles>(server, "/api/v1/admin/custom_styles"),
    );
    Schema.decodeUnknownSync(WorkspaceIconListSchema)(
      await get<WorkspaceIconList>(server, "/api/v1/admin/icons"),
    );
    Schema.decodeUnknownSync(AuditLogPageSchema)(
      await get<AuditLogPage>(server, "/api/v1/admin/audit_log"),
    );
    Schema.decodeUnknownSync(IntegrationsHealthSchema)(
      await get<IntegrationsHealth>(server, "/api/v1/admin/integrations_health"),
    );
  });

  it("list administrators first, without bots or deactivated people", async () => {
    const { server } = harness();
    const { people } = await get<PeoplePage>(server, "/api/v1/admin/people");
    const ids = people.map((person) => person.id);

    expect(ids).not.toContain(BOT_ID);
    expect(ids).not.toContain(DEACTIVATED_ID);
    expect(people.find((person) => person.you)?.id).toBe(VIEWER_ID);

    const firstMember = people.findIndex((person) => person.role === "member");

    expect(people.slice(firstMember).every((person) => person.role === "member")).toBe(true);
  });

  it("change a role and record it in the audit log", async () => {
    const { server } = harness();
    const path = `/api/v1/admin/people/${USER_IDS.maya}`;

    const change = await expectStatus<PersonChange>(
      server,
      "PATCH",
      path,
      { role: "administrator" },
      200,
    );

    expect(change.person.role).toBe("administrator");

    const log = await get<AuditLogPage>(server, "/api/v1/admin/audit_log?action=user.role.change");

    expect(log.entries.map((entry) => entry.target)).toEqual([change.person.name]);
  });

  it("refuse a two-step reset for yourself as the classic page does", async () => {
    const { server } = harness();

    const refused = await expectStatus<Json>(
      server,
      "POST",
      `/api/v1/admin/people/${VIEWER_ID}/two_factor_reset`,
      null,
      422,
    );

    expect(errorOf(refused).tag).toBe("Validation");
  });

  it("name what's wrong with a new icon", async () => {
    const { server } = harness();

    const refused = await expectStatus<Json>(
      server,
      "POST",
      "/api/v1/admin/icons",
      { name: "No Spaces", title: "", signedId: null },
      422,
    );

    expect(errorOf(refused).tag).toBe("Validation");
    expect(Object.keys(field(field(refused, "error"), "fields") ?? {}).sort()).toEqual([
      "image",
      "name",
      "title",
    ]);
  });
});
