/**
 * Curated board tags: the rules the tag editor and the new-post form share with the server
 * (`BoardTag`, `BoardTagPolicy` in crates/db/src/models/thread_tag.rs).
 */
import type { BoardTag } from "../../gen/BoardTag.ts";
import type { BoardTagCatalog } from "../../gen/BoardTagCatalog.ts";
import type { BoardTagCount } from "../../gen/BoardTagCount.ts";
import { caselessEqual } from "../../lib/case-fold.ts";
import { MAX_TAGS, tagsProblem } from "./board-format.ts";

/** A board holds at most 20 curated tags. */
export const MAX_BOARD_TAGS = 20;

/** A curated tag's trimmed name has 1–20 characters. */
export const MAX_BOARD_TAG_NAME = 20;

/** The policy and tags of a board nobody has curated. */
export const EMPTY_CATALOG: Omit<BoardTagCatalog, "roomId"> = {
  tags: [],
  tagsRequired: false,
  defaultBoardTagId: null,
};

/** Whether the server takes two tag names for one: trimmed, under full Unicode case folding. */
export function sameTagName(left: string, right: string): boolean {
  return caselessEqual(left.trim(), right.trim());
}

/** The curated tag a post's tag name stands for, if any. */
export function catalogTag(tags: readonly BoardTag[], name: string): BoardTag | undefined {
  return tags.find((tag) => sameTagName(tag.name, name));
}

/** The catalog's default tag, when it names one still in the catalog. */
export function defaultTag(catalog: Omit<BoardTagCatalog, "roomId">): BoardTag | undefined {
  return catalog.tags.find((tag) => tag.id === catalog.defaultBoardTagId);
}

/** A new post starts with the board's default tag picked, when there is one. */
export function initialSelection(catalog: Omit<BoardTagCatalog, "roomId">): number[] {
  const fallback = defaultTag(catalog);

  return fallback === undefined ? [] : [fallback.id];
}

/** Picked ids that are still in the catalog (someone may have deleted one meanwhile). */
export function keptSelection(selected: readonly number[], tags: readonly BoardTag[]): number[] {
  return selected.filter((id) => tags.some((tag) => tag.id === id));
}

/** Toggles `id` in the picked set, keeping the catalog's order. */
export function toggleSelection(
  selected: readonly number[],
  id: number,
  tags: readonly BoardTag[],
): number[] {
  const on = selected.includes(id);

  return tags.flatMap((tag) => ((tag.id === id ? !on : selected.includes(tag.id)) ? [tag.id] : []));
}

/** What a new post sends as `tags`: the picked labels in catalog order, then the free-text ones. */
export function postTagNames(
  tags: readonly BoardTag[],
  selected: readonly number[],
  freeText: readonly string[],
): string[] {
  const names = tags.flatMap((tag) => (selected.includes(tag.id) ? [tag.name] : []));

  for (const name of freeText) {
    const curated = catalogTag(tags, name);
    const label = curated?.name ?? name;

    if (!names.some((held) => sameTagName(held, label))) names.push(label);
  }

  return names;
}

/** Whether a post's tags hold at least one curated tag. */
export function hasCatalogTag(tags: readonly BoardTag[], names: readonly string[]): boolean {
  return names.some((name) => catalogTag(tags, name) !== undefined);
}

export const REQUIRED_TAG_MESSAGE = "Pick at least one of this board's tags.";

/**
 * The tag the server adds to a post sent with `names`: the default, on a board that requires a
 * tag, when none of `names` is curated (`BoardTagPolicy::post_tags`).
 */
export function addedDefault(
  catalog: Omit<BoardTagCatalog, "roomId">,
  names: readonly string[],
): BoardTag | undefined {
  return catalog.tagsRequired && !hasCatalogTag(catalog.tags, names)
    ? defaultTag(catalog)
    : undefined;
}

/**
 * What the server would refuse about a new post's tags, said for the form: a required tag
 * missing with no default to stand in, more than five in all (the default it adds counts), or
 * a free-text tag outside the classic grammar. Curated labels skip the grammar (they may hold
 * spaces and capitals).
 */
export function postTagsProblem(
  catalog: Omit<BoardTagCatalog, "roomId">,
  names: readonly string[],
): { readonly field: "catalog" | "tags"; readonly message: string } | undefined {
  const added = addedDefault(catalog, names);

  if (
    catalog.tagsRequired &&
    catalog.tags.length > 0 &&
    added === undefined &&
    !hasCatalogTag(catalog.tags, names)
  ) {
    return { field: "catalog", message: REQUIRED_TAG_MESSAGE };
  }

  if (names.length + (added === undefined ? 0 : 1) > MAX_TAGS) {
    return {
      field: "tags",
      message:
        added === undefined
          ? `Use up to ${MAX_TAGS} tags in all.`
          : `Use up to ${MAX_TAGS - 1} tags: ${added.name} is added as well.`,
    };
  }

  const free = names.filter((name) => catalogTag(catalog.tags, name) === undefined);
  const problem = tagsProblem(free);

  return problem === undefined ? undefined : { field: "tags", message: problem };
}

/** The editor's check of a new or renamed tag, before it asks the server. */
export function tagNameProblem(
  tags: readonly BoardTag[],
  name: string,
  editingId: number | null = null,
): string | undefined {
  const trimmed = name.trim();

  if (trimmed === "") return "Give the tag a name.";

  if (Array.from(trimmed).length > MAX_BOARD_TAG_NAME) {
    return `Keep tag names to ${MAX_BOARD_TAG_NAME} characters.`;
  }

  if (editingId === null && tags.length >= MAX_BOARD_TAGS) {
    return `A board holds up to ${MAX_BOARD_TAGS} tags.`;
  }

  const taken = tags.find((tag) => tag.id !== editingId && sameTagName(tag.name, trimmed));

  return taken === undefined ? undefined : `“${taken.name}” is already a tag here.`;
}

/** `ids` with the one at `from` moved to `to` (both clamped to the list). */
export function moveId(ids: readonly number[], from: number, to: number): number[] {
  const next = [...ids];
  const target = Math.max(0, Math.min(next.length - 1, to));

  if (from < 0 || from >= next.length || from === target) return next;

  const [moved] = next.splice(from, 1);

  if (moved !== undefined) next.splice(target, 0, moved);

  return next;
}

/** How many posts carry a tag, from the board's counts (`undefined` when they aren't loaded). */
export function postsUsing(
  counts: readonly BoardTagCount[] | undefined,
  name: string,
): number | undefined {
  if (counts === undefined) return undefined;

  return counts
    .filter((count) => sameTagName(count.name, name))
    .reduce((sum, count) => sum + count.count, 0);
}

export interface DeleteConsequences {
  /** One sentence each, for the confirmation. */
  readonly lines: readonly string[];
  /** Deleting it leaves a required tag with no tag to pick: the requirement goes too. */
  readonly liftsRequirement: boolean;
}

/**
 * What deleting a tag does, in the confirmation's words: it leaves every post that has it, the
 * default goes, and a requirement with nothing left to satisfy it is lifted too.
 */
export function deleteConsequences(
  catalog: Omit<BoardTagCatalog, "roomId">,
  tag: BoardTag,
  posts: number | undefined,
): DeleteConsequences {
  const lines: string[] = [];

  if (posts === undefined) {
    lines.push("Every post that has it loses it.");
  } else if (posts === 0) {
    lines.push("No posts have it yet.");
  } else {
    lines.push(`${posts === 1 ? "1 post loses" : `${posts} posts lose`} it.`);
  }

  if (catalog.defaultBoardTagId === tag.id) {
    lines.push("New posts stop starting with it.");
  }

  const liftsRequirement = catalog.tagsRequired && catalog.tags.length === 1;

  if (liftsRequirement) {
    lines.push("It's the last tag, so new posts stop requiring one.");
  }

  return { lines, liftsRequirement };
}

/** A post's tags for display: curated ones first, in the catalog's order, then the rest as stored. */
export function orderedPostTags(
  tags: readonly BoardTag[],
  names: readonly string[],
): { readonly name: string; readonly tag: BoardTag | undefined }[] {
  const curated = tags.flatMap((tag) => {
    const name = names.find((candidate) => sameTagName(candidate, tag.name));

    return name === undefined ? [] : [{ name, tag }];
  });

  const rest = names.flatMap((name) =>
    catalogTag(tags, name) === undefined ? [{ name, tag: undefined }] : [],
  );

  return [...curated, ...rest];
}
