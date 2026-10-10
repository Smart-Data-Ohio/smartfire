/**
 * The mock's stand-in for the server's Markdown pipeline: paragraphs, line breaks, `- ` lists,
 * fenced code, inline code, **bold**, *em*, links, @mentions (`<@123>`, `@[Exact Name]` and the older
 * `@name`) and `:name:` brand and workspace icons, rendered to the small, already
 * sanitized HTML subset the real `bodyHtml` uses. Everything that isn't markup is escaped.
 */

import { BRAND_ICONS, CUSTOM_ICONS, iconImageUrl } from "./s2/emoji.ts";

/** Someone an `@name` can refer to: their full name or first name, case-insensitively. */
export interface Mentionable {
  readonly id: number;
  readonly name: string;
}

const HTML_ESCAPES = new Map([
  ["&", "&amp;"],
  ["<", "&lt;"],
  [">", "&gt;"],
  ['"', "&quot;"],
  ["'", "&#39;"],
]);

export function escapeHtml(text: string): string {
  return text.replace(/[&<>"']/g, (char) => HTML_ESCAPES.get(char) ?? char);
}

/**
 * A mention exactly as the server renders it (`crates/richtext/src/attachables.rs`
 * `render_mention`): an action-text attachment around the person's card. The UI finds "mentions
 * me" by `data-user-id`.
 */
export function mentionHtml(person: Mentionable): string {
  const id = person.id;
  const name = escapeHtml(person.name);

  return (
    `<action-text-attachment content-type="application/vnd.campfire.mention" sgid="mock-${id}">` +
    `<div class="mention mention--user-${id}" sgid="mock-${id}" data-user-id="${id}">` +
    `<a title="${name}" class="btn avatar" href="/users/${id}">` +
    `<img aria-hidden="true" src="/users/${id}/avatar" width="48" height="48" /></a>` +
    `<button name="button" type="button" class="profile-card-name">${name}</button>` +
    "</div></action-text-attachment>"
  );
}

/** Whether rendered HTML mentions the person with this id. */
export function mentionsUser(html: string, userId: number): boolean {
  return html.includes(`data-user-id="${userId}"`);
}

/**
 * A `:name:` brand or workspace icon as the server renders it (`crates/richtext/src/markdown.rs`
 * `icon_node`); `null` for a name that is neither (emoji shortcodes stay text here).
 */
function iconHtml(name: string): string | null {
  const brand = BRAND_ICONS.find((icon) => icon.name === name);
  const custom = brand === undefined ? CUSTOM_ICONS.find((icon) => icon.name === name) : undefined;
  const icon = brand ?? custom;

  if (icon === undefined) return null;

  const kind = brand === undefined ? "custom" : "brand";

  return `<img class="icon icon--${kind}" src="${iconImageUrl(kind, name)}" alt=":${name}:" title="${escapeHtml(icon.title)}" draggable="false">`;
}

function escapeRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function linkHtml(url: string, label: string): string {
  return `<a href="${escapeHtml(url)}" target="_blank" rel="noopener noreferrer">${escapeHtml(label)}</a>`;
}

/** Inline markup for a run of text that holds no code spans. */
function renderText(text: string, people: readonly Mentionable[]): string {
  const held: string[] = [];

  const hold = (html: string) => `\uE000${held.push(html) - 1}\uE001`;

  // A backslash escape keeps the next punctuation mark literal (`¯\_(ツ)_/¯`).
  let out = text.replace(/\\(<@[1-9][0-9]*>|@\[[^[\]\r\n]+\])/g, (_match, token: string) =>
    hold(escapeHtml(token)),
  );

  out = out.replace(/\\([\\`*_[\]()#+\-.!{}])/g, (_match, char: string) => hold(escapeHtml(char)));

  out = out.replace(
    /\[([^\]\n]+)\]\(((?:https?:\/\/|mailto:)[^\s)]+)\)/g,
    (_match, label: string, url: string) => hold(linkHtml(url, label)),
  );

  out = out.replace(/\bhttps?:\/\/[^\s<\uE000]*[^\s<.,:;"')\]!?\uE000]/g, (url) =>
    hold(linkHtml(url, url)),
  );

  out = out.replace(/:([a-z0-9_]+):/g, (match, name: string) => {
    const html = iconHtml(name);

    return html === null ? match : hold(html);
  });

  out = out.replace(/<@([1-9][0-9]*)>/g, (match, id: string) => {
    const person = people.find((person) => person.id === Number(id));

    return person === undefined ? match : hold(mentionHtml(person));
  });

  // Legacy `@[Exact Name]` resolves only when exactly one person has it.
  out = out.replace(/@\[([^\]\n]+)\]/g, (match, name: string) => {
    const matches = people.filter((person) => person.name === name);
    const [person] = matches;

    return matches.length === 1 && person !== undefined ? hold(mentionHtml(person)) : match;
  });

  const byName = new Map<string, Mentionable>();

  for (const person of people) {
    byName.set(person.name.toLowerCase(), person);

    const first = person.name.split(/\s+/)[0];

    if (first !== undefined && !byName.has(first.toLowerCase())) {
      byName.set(first.toLowerCase(), person);
    }
  }

  if (byName.size > 0) {
    const names = [...byName.keys()].sort((a, b) => b.length - a.length).map(escapeRegExp);
    const pattern = new RegExp(`(^|[^\\w@])@(${names.join("|")})(?![\\w])`, "gi");

    out = out.replace(pattern, (match, lead: string, name: string) => {
      const person = byName.get(name.toLowerCase());

      return person === undefined ? match : `${lead}${hold(mentionHtml(person))}`;
    });
  }

  out = escapeHtml(out)
    .replace(/\*\*([^*\n]+?)\*\*/g, "<strong>$1</strong>")
    .replace(/(^|[^*\w])\*([^*\s](?:[^*\n]*?[^*\s])?)\*(?![*\w])/g, "$1<em>$2</em>")
    .replace(/(^|[^_\w])_([^_\s](?:[^_\n]*?[^_\s])?)_(?![_\w])/g, "$1<em>$2</em>");

  return out.replace(/\uE000(\d+)\uE001/g, (_match, index: string) => held[Number(index)] ?? "");
}

/** Inline markup for one line: code spans first, then everything else. */
function renderInline(line: string, people: readonly Mentionable[]): string {
  return line
    .split(/(`[^`\n]+`)/)
    .map((part) =>
      part.length > 2 && part.startsWith("`") && part.endsWith("`")
        ? `<code>${escapeHtml(part.slice(1, -1))}</code>`
        : renderText(part, people),
    )
    .join("");
}

const LIST_ITEM = /^\s*[-*]\s+/;

/** Renders Markdown to sanitized HTML, mentioning `people` by `@name`. */
export function renderMarkdown(source: string, people: readonly Mentionable[]): string {
  const lines = source.replace(/\r\n?/g, "\n").split("\n");
  const blocks: string[] = [];
  let paragraph: string[] = [];
  let list: string[] = [];

  const flush = () => {
    if (paragraph.length > 0) {
      blocks.push(`<p>${paragraph.map((line) => renderInline(line, people)).join("<br>")}</p>`);
      paragraph = [];
    }

    if (list.length > 0) {
      blocks.push(
        `<ul>${list.map((item) => `<li>${renderInline(item, people)}</li>`).join("")}</ul>`,
      );
      list = [];
    }
  };

  for (let index = 0; index < lines.length; index++) {
    const line = lines[index] ?? "";
    const fence = /^\s*```\s*([\w+-]*)\s*$/.exec(line);

    if (fence !== null) {
      flush();

      const code: string[] = [];

      index++;

      while (index < lines.length && !/^\s*```\s*$/.test(lines[index] ?? "")) {
        code.push(lines[index] ?? "");
        index++;
      }

      const language =
        fence[1] === undefined || fence[1] === "" ? "" : ` class="language-${fence[1]}"`;

      blocks.push(`<pre><code${language}>${escapeHtml(code.join("\n"))}</code></pre>`);
      continue;
    }

    if (line.trim() === "") {
      flush();
      continue;
    }

    if (LIST_ITEM.test(line)) {
      if (paragraph.length > 0) flush();
      list.push(line.replace(LIST_ITEM, ""));
      continue;
    }

    if (list.length > 0) flush();
    paragraph.push(line);
  }

  flush();

  return blocks.join("");
}
