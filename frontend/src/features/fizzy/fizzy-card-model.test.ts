import { describe, expect, it } from "vitest";
import type { FizzyMessageCardForm } from "../../gen/FizzyMessageCardForm.ts";
import { ActionError } from "../../sync/run.ts";
import {
  classifyFailure,
  createBody,
  fieldErrors,
  firstInvalid,
  hasErrors,
  initialDraft,
  localErrors,
  readForm,
  scopeKey,
  VALIDATION_SUMMARY,
} from "./fizzy-card-model.ts";

const form: FizzyMessageCardForm = {
  connected: true,
  boards: [
    { id: "engineering", name: "Engineering" },
    { id: "support", name: "Support" },
  ],
  title: "Signups dipped in week 3",
  description: "Signups dipped in week 3\n\nSource: https://smartfire.test/rooms/1/@2",
  excerpt: "Signups dipped in week 3",
  authorName: "Maya",
  roomDisplayName: "general",
  fizzyUserName: "Riel",
  accountName: "Smart Data",
};

const scope = { roomId: 1, threadId: null, messageId: 2 };

describe("the Fizzy card form", () => {
  it("starts with no board and the server's title and description", () => {
    expect(initialDraft(form)).toEqual({
      boardId: "",
      title: form.title,
      description: form.description,
    });
  });

  it("requires a board and a non-blank title before sending, as classic's browser fields do", () => {
    expect(localErrors({ boardId: "", title: "  ", description: "" })).toEqual({
      boardId: "Choose a board.",
      title: "Enter a title.",
    });
    expect(localErrors({ boardId: "support", title: "Ok", description: "" })).toEqual({});
    expect(hasErrors({})).toBe(false);
    expect(hasErrors({ title: "Enter a title." })).toBe(true);
  });

  it("focuses the first invalid field in form order", () => {
    expect(firstInvalid({ boardId: "x", title: "y" })).toBe("boardId");
    expect(firstInvalid({ title: "y" })).toBe("title");
    expect(firstInvalid({})).toBeNull();
  });

  it("sends the draft as the API's body, leaving the title for the server to strip", () => {
    expect(createBody({ boardId: "support", title: " Fix ", description: "Body" })).toEqual({
      boardId: "support",
      title: " Fix ",
      description: "Body",
    });
  });

  it("keys each source by room, thread and message", () => {
    expect(scopeKey(scope)).toBe("1/-/2");
    expect(scopeKey({ roomId: 1, threadId: 9, messageId: 2 })).toBe("1/9/2");
  });

  it("keeps a validation error's first message for the fields the form shows", () => {
    expect(
      fieldErrors({
        boardId: ["Choose a board.", "Again"],
        title: ["Enter a title."],
        other: ["x"],
      }),
    ).toEqual({ boardId: "Choose a board.", title: "Enter a title." });
  });
});

describe("a failed create", () => {
  it("puts validation errors on their fields, with the server's summary", () => {
    const failure = new ActionError("Validation", VALIDATION_SUMMARY, {
      boardId: ["Choose a board."],
    });

    expect(classifyFailure(failure)).toEqual({
      kind: "validation",
      errors: { boardId: "Choose a board." },
      summary: VALIDATION_SUMMARY,
    });
  });

  it("switches to the disconnected state when there's no connection, or Fizzy rejects it", () => {
    for (const tag of ["FizzyNotConnected", "FizzyTokenRejected"]) {
      expect(classifyFailure(new ActionError(tag, `${tag} message`))).toEqual({
        kind: "disconnected",
        message: `${tag} message`,
      });
    }
  });

  it("shows the card a failed reply left behind instead of offering to create it again", () => {
    const failure = new ActionError(
      "FizzyReplyFailed",
      "Fizzy card #580 created, but the reply could not be posted (Body is too long).",
      {},
      { number: "580", url: "https://fizzy.test/cards/580" },
    );

    expect(classifyFailure(failure)).toEqual({
      kind: "created",
      message: failure.message,
      card: { number: "580", url: "https://fizzy.test/cards/580" },
    });
  });

  it("keeps the form with the server's message for everything else", () => {
    for (const tag of [
      "FizzyReadOnly",
      "FizzyRefused",
      "FizzyUnreachable",
      "FizzyThreadLocked",
      "NetworkError",
    ]) {
      expect(classifyFailure(new ActionError(tag, `${tag} message`))).toEqual({
        kind: "problem",
        message: `${tag} message`,
      });
    }

    expect(classifyFailure(new Error("boom"))).toEqual({ kind: "problem", message: "boom" });
    expect(classifyFailure(new ActionError("FizzyReplyFailed", "No card"))).toEqual({
      kind: "problem",
      message: "No card",
    });
  });
});

describe("reading the form", () => {
  it("returns the form as read", async () => {
    await expect(readForm(async () => form, scope)).resolves.toEqual({ form, notice: null });
  });

  it("reads again after a rejected token disconnected the account, keeping the reason", async () => {
    const reads: number[] = [];

    const read = async () => {
      reads.push(reads.length);

      if (reads.length === 1) {
        throw new ActionError(
          "FizzyTokenRejected",
          "Fizzy rejected the linked token. Reconnect on your profile.",
        );
      }

      return { ...form, connected: false, boards: [] };
    };

    await expect(readForm(read, scope)).resolves.toEqual({
      form: { ...form, connected: false, boards: [] },
      notice: "Fizzy rejected the linked token. Reconnect on your profile.",
    });
    expect(reads).toHaveLength(2);
  });

  it("passes other failures on", async () => {
    const read = async (): Promise<FizzyMessageCardForm> => {
      throw new ActionError("FizzyUnreachable", "Could not reach Fizzy. Try again.");
    };

    await expect(readForm(read, scope)).rejects.toThrow("Could not reach Fizzy. Try again.");
  });
});
