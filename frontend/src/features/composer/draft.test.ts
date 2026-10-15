import { afterEach, describe, expect, it } from "vitest";
import {
  draftKey,
  draftRevision,
  newThreadDraftKey,
  readDraft,
  readMentionDraft,
  writeDraft,
} from "./draft.ts";

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

  it("revises a draft only when its stored text changes", () => {
    const key = draftKey(42, null);
    const other = draftKey(43, null);
    const start = draftRevision(key);

    writeDraft(key, "hello");
    expect(draftRevision(key)).toBe(start + 1);

    // A remount putting the same draft back is not an edit.
    writeDraft(key, "hello");
    expect(draftRevision(key)).toBe(start + 1);

    writeDraft(key, "hello again");
    expect(draftRevision(key)).toBe(start + 2);

    // Editing back to an earlier text is still an edit: a late send must not clear it.
    writeDraft(key, "hello");
    expect(draftRevision(key)).toBe(start + 3);
    expect(draftRevision(other)).toBe(0);
  });

  it("revises a draft when an identical name is picked with a different id", () => {
    const key = draftKey(44, null);
    const mention = { start: 0, text: "@Twin", token: "<@123>" };

    writeDraft(key, "@Twin", [mention]);
    const revision = draftRevision(key);

    writeDraft(key, "@Twin", [mention]);
    expect(draftRevision(key)).toBe(revision);
    writeDraft(key, "@Twin", [{ ...mention, token: "<@456>" }]);
    expect(draftRevision(key)).toBe(revision + 1);
    expect(readMentionDraft(key)).toEqual({
      text: "@Twin",
      mentions: [{ ...mention, token: "<@456>" }],
    });
  });

  it.each([
    '{"start":0}',
    '[{"start":"0","text":"@Twin","token":"<@456>"}]',
    '[{"start":0,"text":"@Other","token":"<@456>"}]',
  ])("ignores corrupt or stale mention metadata: %s", (metadata) => {
    const key = draftKey(45, null);

    writeDraft(key, "@Twin");
    sessionStorage.setItem(`${key}.mentions`, metadata);
    expect(readMentionDraft(key)).toEqual({ text: "@Twin", mentions: [] });
  });

  it("clears a new thread's draft without touching the room's", () => {
    writeDraft(draftKey(12, null), "room text");
    writeDraft(newThreadDraftKey(12, 501), "first reply");

    writeDraft(newThreadDraftKey(12, 501), "");

    expect(readDraft(newThreadDraftKey(12, 501))).toBe("");
    expect(readDraft(draftKey(12, null))).toBe("room text");
  });
});
