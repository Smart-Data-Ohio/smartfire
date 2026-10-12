import { describe, expect, it } from "@effect/vitest";
import { Schema } from "effect";
import { ApiErrorResponse, ThreadAccess } from "../errors.ts";
import { CreateThread, ThreadAccessMembers, ThreadVisibility } from "./thread.ts";

const decodeCreate = Schema.decodeUnknownSync(CreateThread);

const create = {
  parentMessageId: 9001,
  name: null,
  message: {
    clientMessageId: "private-thread",
    markdownSource: "First!",
    replyToMessageId: null,
    replyNotifyAuthor: null,
    attachmentSignedId: null,
  },
};

describe("private-thread wire schemas", () => {
  it("accepts omitted create visibility and both explicit visibility values", () => {
    expect(decodeCreate(create).visibility).toBeUndefined();

    for (const visibility of ["public", "private"]) {
      expect(decodeCreate({ ...create, visibility }).visibility).toBe(visibility);
    }

    expect(() => decodeCreate({ ...create, visibility: "secret" })).toThrow();
    expect(() => Schema.decodeUnknownSync(ThreadVisibility)("secret")).toThrow();
  });

  it("decodes nullable inviters and rejects incomplete access members", () => {
    const members = {
      members: [{ userId: 8, addedById: null, createdAt: "2026-10-12T10:00:00.000Z" }],
      users: [],
    };

    expect(Schema.decodeUnknownSync(ThreadAccessMembers)(members).members[0]?.addedById).toBeNull();
    expect(() =>
      Schema.decodeUnknownSync(ThreadAccessMembers)({ members: [{ userId: 8 }], users: [] }),
    ).toThrow();
  });

  it("decodes every typed refusal reason and rejects unknown reasons", () => {
    const decode = Schema.decodeUnknownSync(ApiErrorResponse);

    for (const reason of [
      "notPrivate",
      "inviteForbidden",
      "removeForbidden",
      "parentRoomRequired",
      "visibilityImmutable",
    ] as const) {
      expect(decode({ error: new ThreadAccess({ message: "Refused", reason }) }).error._tag).toBe(
        "ThreadAccess",
      );
    }

    expect(() =>
      decode({
        error: {
          ...new ThreadAccess({ message: "Refused", reason: "notPrivate" }),
          reason: "other",
        },
      }),
    ).toThrow();
  });
});
