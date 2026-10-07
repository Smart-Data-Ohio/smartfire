import { describe, expect, it } from "@effect/vitest";
import { Schema } from "effect";
import { ApiErrorResponse } from "../errors.ts";
import { messageFixture } from "../testing.ts";
import { CreatedFizzyCard, CreateFizzyCard, FizzyBoard, FizzyMessageCardForm } from "./fizzy.ts";

const FIZZY_REPLY_FAILED = "FizzyReplyFailed";

const form = {
  connected: true,
  boards: [{ id: "03board1", name: "Engineering" }],
  title: "The deploy is broken",
  description: "The deploy is broken\n\nSource: https://chat.test/rooms/12?message_id=1",
  excerpt: "The deploy is broken",
  authorName: "Ada",
  roomDisplayName: "Engineering",
  fizzyUserName: "Ada",
  accountName: "Smart Data",
};

describe("Fizzy wire schemas", () => {
  it("round-trips flat board options and connected/disconnected forms", () => {
    expect(
      Schema.encodeSync(FizzyBoard)(Schema.decodeUnknownSync(FizzyBoard)(form.boards[0])),
    ).toEqual(form.boards[0]);

    for (const value of [
      form,
      { ...form, connected: false, boards: [], fizzyUserName: "", accountName: "" },
    ]) {
      expect(
        Schema.encodeSync(FizzyMessageCardForm)(
          Schema.decodeUnknownSync(FizzyMessageCardForm)(value),
        ),
      ).toEqual(value);
    }
  });

  it("round-trips the create body and created room/thread replies", () => {
    const body = { boardId: "03board1", title: "Fix it", description: "Details" };
    expect(
      Schema.encodeSync(CreateFizzyCard)(Schema.decodeUnknownSync(CreateFizzyCard)(body)),
    ).toEqual(body);

    for (const threadId of [null, 40]) {
      const created = {
        number: "580",
        url: "https://fizzy.test/cards/580",
        notice: "Fizzy card #580 created.",
        message: messageFixture(2, 12, { threadId, replyToMessageId: 1 }),
      };

      expect(
        Schema.encodeSync(CreatedFizzyCard)(Schema.decodeUnknownSync(CreatedFizzyCard)(created)),
      ).toEqual(created);
    }
  });

  it("round-trips each typed failure without dropping a created card's details", () => {
    for (const _tag of [
      "FizzyNotConnected",
      "FizzyUnreachable",
      "FizzyTokenRejected",
      "FizzyReadOnly",
      "FizzyRefused",
      "FizzyThreadLocked",
    ]) {
      const response = { error: { _tag, message: "Classic error text" } };
      expect(
        Schema.encodeSync(ApiErrorResponse)(Schema.decodeUnknownSync(ApiErrorResponse)(response)),
      ).toEqual(response);
    }

    const response = {
      error: {
        _tag: FIZZY_REPLY_FAILED,
        message: "Card created but reply failed",
        number: "580",
        url: "https://fizzy.test/cards/580",
      },
    };

    expect(
      Schema.encodeSync(ApiErrorResponse)(Schema.decodeUnknownSync(ApiErrorResponse)(response)),
    ).toEqual(response);
  });
});
