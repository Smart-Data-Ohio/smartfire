import { describe, expect, it } from "vitest";
import type { SavedItem } from "../../gen/SavedItem.ts";
import { reminderLabel } from "./saved-row.tsx";
import { parseSavedSearch } from "./saved-search.ts";

const NOW = Date.parse("2026-10-06T12:00:00.000Z");

function saved(overrides: Partial<SavedItem>): SavedItem {
  return {
    id: 1,
    messageId: 9001,
    status: "in_progress",
    remindAt: null,
    remindedAt: null,
    createdAt: "2026-10-06T10:00:00.000Z",
    ...overrides,
  };
}

describe("parseSavedSearch", () => {
  it("keeps done and all, and drops the default and junk", () => {
    expect(parseSavedSearch({ status: "done" })).toEqual({ status: "done" });
    expect(parseSavedSearch({ status: "all" })).toEqual({ status: "all" });
    expect(parseSavedSearch({ status: "in_progress" })).toEqual({ status: undefined });
    expect(parseSavedSearch({})).toEqual({ status: undefined });
  });
});

describe("reminderLabel", () => {
  it("says nothing without a reminder", () => {
    expect(reminderLabel(saved({}), NOW)).toBeNull();
  });

  it("says when it reminds, or that it did", () => {
    expect(reminderLabel(saved({ remindAt: "2026-10-07T13:00:00.000Z" }), NOW)).toMatch(
      /^Reminds /,
    );
    expect(reminderLabel(saved({ remindedAt: "2026-10-06T10:00:00.000Z" }), NOW)).toMatch(
      /^Reminded 2 hours? ago|^Reminded 2h ago/,
    );
  });
});
