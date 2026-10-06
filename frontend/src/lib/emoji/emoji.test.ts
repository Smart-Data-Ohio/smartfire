import { beforeAll, beforeEach, describe, expect, it } from "vitest";
import { buildEmojiData, type EmojiData, loadEmojiData } from "./data.ts";
import { layoutSections, moveActive, pickerSections, rowStartsOf } from "./picker-layout.ts";
import { quickReactions, recordRecentEmoji, resetRecentEmoji, withRecent } from "./recent.ts";
import { normalizeQuery, searchEmoji } from "./search.ts";

let data: EmojiData;

beforeAll(async () => {
  data = await loadEmojiData();
});

const chars = (list: readonly { char: string }[]) => list.map((emoji) => emoji.char);

describe("emoji search", () => {
  it("normalizes what people type", () => {
    expect(normalizeQuery("  :Thumbs_Up: ")).toBe("thumbs_up");
    expect(normalizeQuery("::")).toBe("");
  });

  it("puts an exact shortcode first", () => {
    expect(chars(searchEmoji(data.all, "fire"))[0]).toBe("🔥");
    expect(chars(searchEmoji(data.all, ":tada:"))[0]).toBe("🎉");
  });

  it("matches words of the name and keywords", () => {
    expect(chars(searchEmoji(data.all, "rocket"))).toContain("🚀");
    expect(chars(searchEmoji(data.all, "party"))).toContain("🎉");
  });

  it("needs every word of a multi-word query", () => {
    const results = chars(searchEmoji(data.all, "heart eyes"));

    expect(results).toContain("😍");
    expect(results).not.toContain("❤️");
  });

  it("answers nothing for an empty query or a miss, and honours the limit", () => {
    expect(searchEmoji(data.all, "   ")).toEqual([]);
    expect(searchEmoji(data.all, "zzzqqq")).toEqual([]);
    expect(searchEmoji(data.all, "a", 5)).toHaveLength(5);
  });

  it("indexes by character and alias", () => {
    const built = buildEmojiData({
      groups: [{ id: "smileys", emoji: [["😀", "grinning", "grinning face", "smile happy"]] }],
    });

    expect(built.byChar.get("😀")?.name).toBe("grinning face");
    expect(built.byAlias.get("grinning")?.char).toBe("😀");
    expect(built.all[0]?.keywords).toEqual(["smile", "happy"]);
  });
});

describe("picker layout", () => {
  const choice = (content: string) => ({
    choice: { content, title: content, imageUrl: null },
    shortcode: null,
  });

  it("lays sections out as a header and rows of columns, skipping empty ones", () => {
    const layout = layoutSections(
      [
        { id: "recent", label: "Frequently used", choices: ["a", "b", "c", "d", "e"].map(choice) },
        { id: "custom", label: "Custom", choices: [] },
        { id: "smileys", label: "Smileys", choices: ["f", "g"].map(choice) },
      ],
      3,
    );

    expect(layout.rows.map((row) => row.kind)).toEqual([
      "header",
      "cells",
      "cells",
      "header",
      "cells",
    ]);
    expect(layout.cells.map((cell) => cell.index)).toEqual([0, 1, 2, 3, 4, 5, 6]);
    expect(layout.sectionRows.get("smileys")).toBe(3);
    expect(layout.sectionRows.has("custom")).toBe(false);
    expect(rowStartsOf(layout.rows)).toEqual([0, 3, 5]);
  });

  it("moves across and between rows, landing on a short row's last cell", () => {
    const starts = [0, 3, 5];

    expect(moveActive(0, "ArrowRight", 7, 3, starts)).toBe(1);
    expect(moveActive(0, "ArrowLeft", 7, 3, starts)).toBe(0);
    expect(moveActive(2, "ArrowDown", 7, 3, starts)).toBe(4);
    expect(moveActive(4, "ArrowDown", 7, 3, starts)).toBe(6);
    expect(moveActive(6, "ArrowUp", 7, 3, starts)).toBe(4);
    expect(moveActive(1, "ArrowUp", 7, 3, starts)).toBe(1);
    expect(moveActive(1, "Enter", 7, 3, starts)).toBeNull();
    expect(moveActive(0, "ArrowDown", 0)).toBeNull();
  });

  it("shows one Results section for a query, icons first", () => {
    const shipit = { content: ":shipit:", title: "Ship it", imageUrl: "/icons/shipit.png" };
    const [results] = pickerSections(data, "ship", [], [shipit]);

    expect(results?.id).toBe("results");
    expect(results?.choices[0]?.choice).toEqual(shipit);
    expect(results?.choices[0]?.shortcode).toBe("shipit");
    expect(results?.choices.some((entry) => entry.choice.content === "🚢")).toBe(true);
  });

  it("without a query shows recent, every category and custom", () => {
    const sections = pickerSections(
      data,
      "",
      [{ content: "🚀", title: "Rocket", imageUrl: null }],
      [],
    );

    expect(sections[0]?.id).toBe("recent");
    expect(sections.at(-1)?.id).toBe("custom");
    expect(sections).toHaveLength(data.groups.length + 2);
  });
});

describe("recent emoji", () => {
  beforeEach(() => resetRecentEmoji());

  it("falls back to the default quick reactions", () => {
    expect(quickReactions([]).map((entry) => entry.content)).toEqual(["👍", "❤️", "😂"]);
  });

  it("puts the latest pick first, without duplicates, up to the limit", () => {
    const rocket = { content: "🚀", title: "Rocket", imageUrl: null };
    const eyes = { content: "👀", title: "Eyes", imageUrl: null };

    expect(withRecent([rocket, eyes], rocket)).toEqual([rocket, eyes]);
    expect(withRecent([rocket], eyes)).toEqual([eyes, rocket]);
    expect(withRecent([rocket], eyes, 1)).toEqual([eyes]);
  });

  it("feeds recorded picks into the quick reactions", () => {
    recordRecentEmoji({ content: "🚀", title: "Rocket", imageUrl: null });

    expect(localStorage.getItem("smartfire:emoji-recent")).toContain("🚀");
  });

  it("fills quick reactions from recent first", () => {
    const rocket = { content: "🚀", title: "Rocket", imageUrl: null };

    expect(quickReactions([rocket]).map((entry) => entry.content)).toEqual(["🚀", "👍", "❤️"]);
  });
});
