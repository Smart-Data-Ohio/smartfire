import { describe, expect, it } from "vitest";
import { validation as earlierValidation } from "../http.ts";
import { invalid, invalidBody, sentence, validation } from "./http.ts";

describe("S4 validation envelopes", () => {
  it("keeps bare model messages and renames only the wire fields", () => {
    expect(
      invalid(
        [
          ["work_owner", "requires work tracking"],
          ["open_questions", "are limited to 10 per handoff"],
        ],
        { work_owner: "ownerId" },
      ).error,
    ).toEqual({
      _tag: expect.stringMatching(/^Validation$/),
      message: "Work owner requires work tracking, Open questions are limited to 10 per handoff",
      fields: {
        ownerId: ["requires work tracking"],
        openQuestions: ["are limited to 10 per handoff"],
      },
    });
    expect(validation("before", "is invalid").error).toEqual({
      _tag: expect.stringMatching(/^Validation$/),
      message: "Before is invalid",
      fields: { before: ["is invalid"] },
    });
  });

  it("preserves classic refusal sentences and undecodable bodies", () => {
    expect(sentence("base", "Request is already approved").error).toEqual({
      _tag: expect.stringMatching(/^Validation$/),
      message: "Request is already approved",
      fields: { base: ["Request is already approved"] },
    });
    expect(invalidBody("missing field receiverAgentId").error).toEqual({
      _tag: expect.stringMatching(/^Validation$/),
      message: "The request body isn't valid: missing field receiverAgentId",
      fields: {},
    });
  });

  it("leaves the S2 and S3 validation envelope unchanged", () => {
    expect(earlierValidation("name", "Name can't be blank").error).toEqual({
      _tag: expect.stringMatching(/^Validation$/),
      message: "Validation failed: Name can't be blank",
      fields: { name: ["Name can't be blank"] },
    });
  });
});
