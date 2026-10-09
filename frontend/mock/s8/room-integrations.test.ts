import { describe, expect, it } from "vitest";
import type { GithubSubscription } from "../../src/gen/GithubSubscription.ts";
import type { GithubSubscriptionList } from "../../src/gen/GithubSubscriptionList.ts";
import type { InboundEmail } from "../../src/gen/InboundEmail.ts";
import type { RoomForm } from "../../src/gen/RoomForm.ts";
import { field } from "../json.ts";
import { errorOf, expectStatus, get, harness, send } from "../s2/testing.ts";
import { SEED_IDS } from "../server.ts";
import { GITHUB_BOT_ID } from "./room-integrations.ts";

const { rooms, boards } = SEED_IDS;

const LAUNCH_ADDRESS = "room-a1b2c3d4e5f67890a1b2c3d4e5f67890@mail.campfire.test";

describe("room integration mock", () => {
  it("subscribes, updates and unsubscribes, and refuses the people classic refuses", async () => {
    const { server } = harness();
    const roomId = rooms.launchPlanning;
    const path = `/api/v1/rooms/${roomId}/github_subscriptions`;

    const empty = await get<GithubSubscriptionList>(server, path);

    expect(empty.subscriptions).toEqual([]);
    expect(empty.administrator).toBe(true);
    expect(empty.connectPath).toBe("/github/app/connect");
    expect(empty.events).toHaveLength(6);
    expect(empty.events.find((event) => event.key === "opened")?.selectedByDefault).toBe(true);
    expect(empty.events.find((event) => event.key === "closed")?.selectedByDefault).toBe(false);

    const created = await expectStatus<GithubSubscription>(
      server,
      "POST",
      path,
      { fullName: "Rails/Rails", events: [], skipAccessCheck: true },
      201,
    );

    expect(created.fullName).toBe("rails/rails");
    expect(created.events).toEqual(["opened", "merged", "review_requested", "checks_failed"]);

    const joined = await get<RoomForm>(server, `/api/v1/rooms/${roomId}/edit`);

    expect(joined.userIds).toContain(GITHUB_BOT_ID);

    const duplicate = await send(server, "POST", path, {
      fullName: "rails/rails",
      events: ["opened"],
      skipAccessCheck: false,
    });

    expect(duplicate.status).toBe(422);
    expect(errorOf(duplicate.json).message).toBe(
      "Could not subscribe: Owner has already been taken.",
    );

    const blank = await send(server, "POST", path, {
      fullName: " ",
      events: ["opened"],
      skipAccessCheck: false,
    });

    expect(blank.status).toBe(422);
    expect(errorOf(blank.json).message).toContain("can't be blank");

    const unknown = await send(server, "POST", path, {
      fullName: "campfire/campfire",
      events: ["deployed"],
      skipAccessCheck: false,
    });

    expect(unknown.status).toBe(422);
    expect(errorOf(unknown.json).message).toContain("must be a subset");

    const updated = await expectStatus<GithubSubscription>(
      server,
      "PATCH",
      `${path}/${created.id}`,
      { events: ["closed"] },
      200,
    );

    expect(updated.events).toEqual(["closed"]);

    const emptied = await send(server, "PATCH", `${path}/${created.id}`, { events: [] });

    expect(emptied.status).toBe(422);
    expect(errorOf(emptied.json).message).toContain("at least one event");

    await expectStatus(server, "DELETE", `${path}/${created.id}`, null, 200);
    expect((await get<GithubSubscriptionList>(server, path)).subscriptions).toEqual([]);
    expect((await get<RoomForm>(server, `/api/v1/rooms/${roomId}/edit`)).userIds).not.toContain(
      GITHUB_BOT_ID,
    );

    expect(
      (await send(server, "GET", `/api/v1/rooms/${rooms.dmMaya}/github_subscriptions`)).status,
    ).toBe(404);
    expect(
      (await send(server, "GET", `/api/v1/rooms/${boards.roomId}/github_subscriptions`)).status,
    ).toBe(200);

    await send(server, "POST", "/__mock/viewer-role", { role: "member" });
    const refused = await send(server, "GET", path);

    expect(refused.status).toBe(403);
    expect(errorOf(refused.json).message).toBe("Not allowed");
  });

  it("shows the seeded address, rotates it, and hides inbound email on boards and directs", async () => {
    const { server } = harness();
    const path = `/api/v1/rooms/${rooms.launchPlanning}/inbound_email`;
    const before = await get<InboundEmail>(server, path);

    expect(before).toEqual({ enabled: true, address: LAUNCH_ADDRESS });

    const rotated = await expectStatus<InboundEmail>(server, "POST", path, null, 200);

    expect(rotated.enabled).toBe(true);
    expect(rotated.address).toMatch(/^room-[0-9a-f]{32}@mail\.campfire\.test$/);
    expect(rotated.address).not.toBe(LAUNCH_ADDRESS);

    const again = await expectStatus<InboundEmail>(server, "POST", path, null, 200);

    expect(again.address).not.toBe(rotated.address);

    const lounge = await get<InboundEmail>(server, `/api/v1/rooms/${rooms.lounge}/inbound_email`);

    expect(lounge).toEqual({ enabled: true, address: null });

    expect((await send(server, "GET", `/api/v1/rooms/${rooms.dmMaya}/inbound_email`)).status).toBe(
      404,
    );
    expect((await send(server, "GET", `/api/v1/rooms/${boards.roomId}/inbound_email`)).status).toBe(
      404,
    );

    await send(server, "POST", "/__mock/viewer-role", { role: "member" });
    const refused = await send(server, "POST", path);

    expect(refused.status).toBe(403);
    expect(field(field(refused.json, "error"), "_tag")).toBe("Forbidden");
  });
});
