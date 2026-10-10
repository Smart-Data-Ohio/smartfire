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
import type { DirectUpload } from "../../src/gen/DirectUpload.ts";
import type { IconList } from "../../src/gen/IconList.ts";
import type { IntegrationsHealth } from "../../src/gen/IntegrationsHealth.ts";
import type { MessageReactions } from "../../src/gen/MessageReactions.ts";
import type { PeoplePage } from "../../src/gen/PeoplePage.ts";
import type { PersonChange } from "../../src/gen/PersonChange.ts";
import type { Workspace } from "../../src/gen/Workspace.ts";
import type { WorkspaceIconList } from "../../src/gen/WorkspaceIconList.ts";
import { field, type Json } from "../json.ts";
import { BOT_ID, DEACTIVATED_ID, USER_IDS, VIEWER_ID } from "../seed.ts";
import { type MockServer, SEED_IDS } from "../server.ts";
import { animatedIconGif, utf8 } from "./assets.ts";
import { md5Base64 } from "./digest.ts";
import { errorOf, expectStatus, get, harness, send } from "./testing.ts";

/** Uploads `bytes` as the browser's direct upload does; the signed id. */
async function uploaded(server: MockServer, filename: string, type: string, bytes: Uint8Array) {
  const upload = await expectStatus<DirectUpload>(
    server,
    "POST",
    "/api/v1/uploads",
    { filename, byteSize: bytes.length, checksum: md5Base64(bytes), contentType: type },
    200,
  );

  await server.handleBinary({
    method: "PUT",
    path: upload.uploadUrl,
    headers: { "Content-Type": type },
    bytes,
  });

  return upload.signedId;
}

function addIcon<T>(server: MockServer, name: string, signedId: string, status: number) {
  return expectStatus<T>(
    server,
    "POST",
    "/api/v1/admin/icons",
    { name, title: name, signedId },
    status,
  );
}

const STILL_SVG = utf8(
  '<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64"/></svg>',
);

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

  it("take an animated icon: counted against the limit, reactable, served with a still", async () => {
    const { server } = harness();
    const signedId = await uploaded(server, "dance.gif", "image/gif", animatedIconGif());
    const list = await addIcon<WorkspaceIconList>(server, "dance", signedId, 200);

    expect(list.animatedLimit).toBe(250);
    expect(list.animatedUsage).toBe(1);
    expect(list.icons.find((icon) => icon.name === "dance")).toMatchObject({
      animated: true,
      imageUrl: "/icons/dance",
      stillUrl: "/icons/dance?still=1",
    });

    const catalog = await get<IconList>(server, "/api/v1/icons");

    expect(catalog.icons.find((icon) => icon.name === "dance")).toMatchObject({
      kind: "custom",
      animated: true,
    });

    const fetchIcon = (path: string) => server.handleBinary({ method: "GET", path, bytes: null });

    expect((await fetchIcon("/icons/dance")).contentType).toBe("image/gif");
    expect((await fetchIcon("/icons/dance?still=1")).contentType).toBe("image/svg+xml");

    const reacted = await expectStatus<MessageReactions>(
      server,
      "POST",
      `/api/v1/messages/${SEED_IDS.messages.generalChart}/boosts`,
      { content: ":dance:" },
      200,
    );

    expect(reacted.reactions.at(-1)).toMatchObject({
      content: ":dance:",
      title: "dance",
      imageUrl: "/icons/dance",
    });
  });

  it("refuse an animated icon past the limit, and keep static icons unlimited", async () => {
    const { server } = harness();

    await send(server, "POST", "/__mock/animated-icon-limit", { limit: 0 });

    const gif = await uploaded(server, "dance.gif", "image/gif", animatedIconGif());
    const refused = await addIcon<Json>(server, "dance", gif, 422);

    expect(field(field(field(refused, "error"), "fields"), "image")).toEqual([
      "animated emoji capacity reached (limit: 0)",
    ]);

    const svg = await uploaded(server, "still.svg", "image/svg+xml", STILL_SVG);
    const list = await addIcon<WorkspaceIconList>(server, "still", svg, 200);

    expect(list).toMatchObject({ animatedLimit: 0, animatedUsage: 0 });
    expect(list.icons.find((icon) => icon.name === "still")?.animated).toBe(false);
  });
});
