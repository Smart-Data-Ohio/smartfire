import { afterEach, describe, expect, it } from "vitest";
import { draftKey, newThreadDraftKey, readDraft, writeDraft } from "./draft.ts";

describe("composer drafts", () => {
  afterEach(() => sessionStorage.clear());

  it("keeps a room's, a thread's and a new thread's drafts apart", () => {
    writeDraft(draftKey(12, null), "room text");
    writeDraft(draftKey(12, 88), "thread text");
    writeDraft(newThreadDraftKey(12, 501), "first reply");

    expect(readDraft(draftKey(12, null))).toBe("room text");
    expect(readDraft(draftKey(12, 88))).toBe("thread text");
    expect(readDraft(newThreadDraftKey(12, 501))).toBe("first reply");
  });

  it("clears a new thread's draft without touching the room's", () => {
    writeDraft(draftKey(12, null), "room text");
    writeDraft(newThreadDraftKey(12, 501), "first reply");

    writeDraft(newThreadDraftKey(12, 501), "");

    expect(readDraft(newThreadDraftKey(12, 501))).toBe("");
    expect(readDraft(draftKey(12, null))).toBe("room text");
  });
});
