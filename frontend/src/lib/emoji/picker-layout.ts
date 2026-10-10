import { type EmojiData, emojiTitle } from "./data.ts";
import type { EmojiChoice } from "./recent.ts";
import { normalizeQuery, searchEmoji } from "./search.ts";

/** Cells per row in the picker grid. */
export const PICKER_COLUMNS = 9;

/** A tab and the grid section it scrolls to. */
export type SectionId = "recent" | "results" | "custom" | EmojiData["groups"][number]["id"];

export interface PickerCell {
  readonly choice: EmojiChoice;
  /** The shortcode shown in the preview footer (`:thumbsup:`), when there is one. */
  readonly shortcode: string | null;
  /** Position among every cell, for keyboard movement and `aria-activedescendant`. */
  readonly index: number;
}

export type PickerRow =
  | {
      readonly kind: "header";
      readonly key: string;
      readonly section: SectionId;
      readonly label: string;
    }
  | {
      readonly kind: "cells";
      readonly key: string;
      readonly section: SectionId;
      readonly cells: readonly PickerCell[];
    };

export interface PickerSection {
  readonly id: SectionId;
  readonly label: string;
  readonly choices: readonly { readonly choice: EmojiChoice; readonly shortcode: string | null }[];
}

export interface PickerLayout {
  readonly rows: readonly PickerRow[];
  readonly cells: readonly PickerCell[];
  /** The row each section's header sits on. */
  readonly sectionRows: ReadonlyMap<SectionId, number>;
}

/** Lays sections out as a header row plus rows of `columns` cells each; empty sections vanish. */
export function layoutSections(
  sections: readonly PickerSection[],
  columns = PICKER_COLUMNS,
): PickerLayout {
  const rows: PickerRow[] = [];
  const cells: PickerCell[] = [];
  const sectionRows = new Map<SectionId, number>();

  for (const section of sections) {
    if (section.choices.length === 0) {
      continue;
    }

    sectionRows.set(section.id, rows.length);
    rows.push({
      kind: "header",
      key: `h-${section.id}`,
      section: section.id,
      label: section.label,
    });

    for (let start = 0; start < section.choices.length; start += columns) {
      const rowCells = section.choices.slice(start, start + columns).map((entry) => {
        const cell = { ...entry, index: cells.length };

        cells.push(cell);

        return cell;
      });

      rows.push({
        kind: "cells",
        key: `r-${section.id}-${start}`,
        section: section.id,
        cells: rowCells,
      });
    }
  }

  return { rows, cells, sectionRows };
}

function emojiEntry(data: EmojiData, char: string) {
  const emoji = data.byChar.get(char);

  return {
    choice: {
      content: char,
      title: emoji === undefined ? char : emojiTitle(emoji),
      imageUrl: null,
    },
    shortcode: emoji?.aliases[0] ?? null,
  };
}

function iconShortcode(choice: EmojiChoice): string | null {
  return /^:[a-z0-9_+-]+:$/.test(choice.content) ? choice.content.slice(1, -1) : null;
}

/**
 * The picker's sections: with no query, Recent, every category and the workspace's own icons;
 * with one, a single "Results" section of matching emoji and icons.
 */
export function pickerSections(
  data: EmojiData,
  query: string,
  recent: readonly EmojiChoice[],
  custom: readonly EmojiChoice[],
): PickerSection[] {
  const normalized = normalizeQuery(query);

  if (normalized !== "") {
    const icons = custom.filter(
      (choice) =>
        choice.content.includes(normalized) || choice.title.toLowerCase().includes(normalized),
    );

    return [
      {
        id: "results",
        label: "Results",
        choices: [
          ...icons.map((choice) => ({ choice, shortcode: iconShortcode(choice) })),
          ...searchEmoji(data.all, normalized).map((emoji) => emojiEntry(data, emoji.char)),
        ],
      },
    ];
  }

  return [
    {
      id: "recent",
      label: "Frequently used",
      // A recent icon shows as the current catalog has it (its still included), when it's there.
      choices: recent.map((choice) =>
        choice.imageUrl === null
          ? emojiEntry(data, choice.content)
          : {
              choice: custom.find((icon) => icon.content === choice.content) ?? choice,
              shortcode: iconShortcode(choice),
            },
      ),
    },
    ...data.groups.map((group) => ({
      id: group.id,
      label: group.label,
      choices: group.emoji.map((emoji) => emojiEntry(data, emoji.char)),
    })),
    {
      id: "custom",
      label: "Custom",
      choices: custom.map((choice) => ({ choice, shortcode: iconShortcode(choice) })),
    },
  ];
}

/**
 * Where the active cell goes for an arrow key: across a row, or a row up or down (to the last
 * cell when the row below is short). `null` for any other key.
 */
export function moveActive(
  index: number,
  key: string,
  total: number,
  columns = PICKER_COLUMNS,
  rowStarts: readonly number[] = [],
): number | null {
  if (total === 0) {
    return null;
  }

  switch (key) {
    case "ArrowRight":
      return Math.min(total - 1, index + 1);
    case "ArrowLeft":
      return Math.max(0, index - 1);
    case "ArrowDown":
      return verticalMove(index, 1, total, columns, rowStarts);
    case "ArrowUp":
      return verticalMove(index, -1, total, columns, rowStarts);
    default:
      return null;
  }
}

/** Row-aware vertical move: sections start new rows, so a column is counted from its row start. */
function verticalMove(
  index: number,
  direction: 1 | -1,
  total: number,
  columns: number,
  rowStarts: readonly number[],
): number {
  if (rowStarts.length === 0) {
    return Math.min(total - 1, Math.max(0, index + direction * columns));
  }

  const row = rowStarts.findLastIndex((start) => start <= index);
  const column = index - (rowStarts[row] ?? 0);
  const target = rowStarts[row + direction];

  if (target === undefined) {
    return index;
  }

  const targetEnd = (rowStarts[row + direction + 1] ?? total) - 1;

  return Math.min(target + column, targetEnd);
}

/** The first cell index of every cells row, for row-aware vertical moves. */
export function rowStartsOf(rows: readonly PickerRow[]): number[] {
  return rows.flatMap((row) => (row.kind === "cells" ? [row.cells[0]?.index ?? 0] : []));
}
