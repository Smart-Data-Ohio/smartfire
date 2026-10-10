import {
  createHighlighterCore,
  type HighlighterCore,
  type LanguageRegistration,
  type ThemeRegistration,
} from "shiki/core";
import { createJavaScriptRegexEngine } from "shiki/engine/javascript";
import darkPlus from "shiki/themes/dark-plus.mjs";
import {
  type CodeToken,
  type HighlightResult,
  isTokenKind,
  PLAIN_RESULT,
  type TokenKind,
} from "./kinds.ts";

/**
 * Colours a code block as classic does (web/script/code-highlighter/highlighter.mjs): Shiki 4.4.3's
 * TextMate grammars on its JavaScript regex engine, with the same languages, aliases and detection.
 * Classic paints Shiki's Light+/Dark+ colours; here Dark+'s scope rules are kept but each colour
 * becomes a token kind, painted from the design tokens (kinds.ts).
 *
 * This module is the lazy part: the code-highlight worker (worker.ts) runs it, so Shiki and every
 * grammar stay out of the app's entry chunks, and each grammar loads only when a block needs it.
 */
type Grammar = () => Promise<{ readonly default: readonly LanguageRegistration[] }>;

const GRAMMARS: ReadonlyMap<string, Grammar> = new Map<string, Grammar>([
  ["c", () => import("shiki/langs/c.mjs")],
  ["cpp", () => import("shiki/langs/cpp.mjs")],
  ["cpp-macro", () => import("shiki/langs/cpp.mjs")],
  ["csharp", () => import("shiki/langs/csharp.mjs")],
  ["css", () => import("shiki/langs/css.mjs")],
  ["diff", () => import("shiki/langs/diff.mjs")],
  ["docker", () => import("shiki/langs/docker.mjs")],
  ["glsl", () => import("shiki/langs/glsl.mjs")],
  ["go", () => import("shiki/langs/go.mjs")],
  ["graphql", () => import("shiki/langs/graphql.mjs")],
  ["haml", () => import("shiki/langs/haml.mjs")],
  ["html", () => import("shiki/langs/html.mjs")],
  ["java", () => import("shiki/langs/java.mjs")],
  ["javascript", () => import("shiki/langs/javascript.mjs")],
  ["json", () => import("shiki/langs/json.mjs")],
  ["jsx", () => import("shiki/langs/jsx.mjs")],
  ["lua", () => import("shiki/langs/lua.mjs")],
  ["powershell", () => import("shiki/langs/powershell.mjs")],
  ["python", () => import("shiki/langs/python.mjs")],
  ["regexp", () => import("shiki/langs/regexp.mjs")],
  ["ruby", () => import("shiki/langs/ruby.mjs")],
  ["rust", () => import("shiki/langs/rust.mjs")],
  ["shellscript", () => import("shiki/langs/shellscript.mjs")],
  ["sql", () => import("shiki/langs/sql.mjs")],
  ["tsx", () => import("shiki/langs/tsx.mjs")],
  ["typescript", () => import("shiki/langs/typescript.mjs")],
  ["xml", () => import("shiki/langs/xml.mjs")],
  ["yaml", () => import("shiki/langs/yaml.mjs")],
]);

/**
 * Every label classic colours, lower-cased, and its grammar: classic's own aliases, then each
 * bundled grammar's name and aliases, the grammars they embed included (lua, graphql, haml, glsl,
 * regexp and cpp-macro come in with ruby, html and cpp). Any other label stays plain.
 */
export const LANGUAGE_ALIASES: ReadonlyMap<string, string> = new Map([
  ["bash", "shellscript"],
  ["sh", "shellscript"],
  ["shell", "shellscript"],
  ["zsh", "shellscript"],
  ["shellscript", "shellscript"],
  ["c#", "csharp"],
  ["cs", "csharp"],
  ["csharp", "csharp"],
  ["c++", "cpp"],
  ["cpp", "cpp"],
  ["cpp-macro", "cpp-macro"],
  ["c", "c"],
  ["dockerfile", "docker"],
  ["docker", "docker"],
  ["ps1", "powershell"],
  ["ps", "powershell"],
  ["pwsh", "powershell"],
  ["powershell", "powershell"],
  ["regexp", "regexp"],
  ["regex", "regexp"],
  ["glsl", "glsl"],
  ["css", "css"],
  ["diff", "diff"],
  ["go", "go"],
  ["javascript", "javascript"],
  ["js", "javascript"],
  ["cjs", "javascript"],
  ["mjs", "javascript"],
  ["html", "html"],
  ["java", "java"],
  ["json", "json"],
  ["jsx", "jsx"],
  ["python", "python"],
  ["py", "python"],
  ["haml", "haml"],
  ["xml", "xml"],
  ["sql", "sql"],
  ["typescript", "typescript"],
  ["ts", "typescript"],
  ["cts", "typescript"],
  ["mts", "typescript"],
  ["tsx", "tsx"],
  ["graphql", "graphql"],
  ["gql", "graphql"],
  ["lua", "lua"],
  ["yaml", "yaml"],
  ["yml", "yaml"],
  ["ruby", "ruby"],
  ["rb", "ruby"],
  ["rust", "rust"],
  ["rs", "rust"],
]);

/** Dark+'s colours (lower-cased) and the kind each paints; the rest stay the block's text colour. */
const KIND_BY_COLOR: ReadonlyMap<string, TokenKind> = new Map([
  ["#6a9955", "comment"],
  ["#569cd6", "keyword"],
  ["#c586c0", "control"],
  ["#ce9178", "string"],
  ["#d16969", "regexp"],
  ["#646695", "regexp"],
  ["#d7ba7d", "special"],
  ["#b5cea8", "number"],
  ["#9cdcfe", "variable"],
  ["#4fc1ff", "constant"],
  ["#dcdcaa", "function"],
  ["#4ec9b0", "type"],
  ["#808080", "punctuation"],
  ["#f44747", "invalid"],
]);

const PLAIN = "var(--code-plain)";

const kindColor = (kind: TokenKind) => `var(--code-${kind})`;

const KIND_COLOR = /^var\(--code-([a-z]+)\)$/;

/** Dark+'s scope rules with each colour swapped for a kind's variable (read back in `tokens`). */
const THEME_NAME = "smartfire";

const THEME: ThemeRegistration = {
  name: THEME_NAME,
  type: "dark",
  colors: { "editor.foreground": PLAIN, "editor.background": "transparent" },
  tokenColors: [
    ...(darkPlus.tokenColors ?? []).map((rule) => {
      const foreground = rule.settings.foreground?.toLowerCase();

      if (foreground === undefined) {
        // Font-style-only rules (markdown emphasis) leave the colour to the enclosing scope.
        return { ...rule, settings: {} };
      }

      const kind = KIND_BY_COLOR.get(foreground);

      return { ...rule, settings: { foreground: kind === undefined ? PLAIN : kindColor(kind) } };
    }),
    // Dark+ paints a diff's lines as numbers and strings; these read as added and removed.
    { scope: ["markup.inserted"], settings: { foreground: kindColor("inserted") } },
    { scope: ["markup.deleted"], settings: { foreground: kindColor("deleted") } },
  ],
};

let highlighter: Promise<HighlighterCore> | undefined;

const loaded = new Map<string, Promise<void>>();

function core(): Promise<HighlighterCore> {
  highlighter ??= createHighlighterCore({
    themes: [THEME],
    langs: [],
    engine: createJavaScriptRegexEngine(),
  });

  return highlighter;
}

async function withGrammar(language: string): Promise<HighlighterCore> {
  const engine = await core();
  const grammar = GRAMMARS.get(language);

  if (grammar !== undefined && !loaded.has(language)) {
    loaded.set(
      language,
      grammar().then((module) => engine.loadLanguage(...module.default)),
    );
  }

  await loaded.get(language);

  return engine;
}

/** The language a block is coloured as: its label's grammar, else a guess, else none. */
export async function resolveLanguage(
  source: string,
  label: string | undefined,
): Promise<string | null> {
  // An unknown explicit label, or a text fence, must never fall back to guessing.
  const name =
    label === undefined
      ? (await import("./detect.ts")).detectLanguage(source)
      : label.toLowerCase();

  return LANGUAGE_ALIASES.get(name ?? "") ?? null;
}

/** The block's coloured runs, adjacent runs of one kind merged; plain text is left out. */
export async function highlight(
  source: string,
  label: string | undefined,
): Promise<HighlightResult> {
  const language = await resolveLanguage(source, label);

  if (language === null) {
    return PLAIN_RESULT;
  }

  const engine = await withGrammar(language);
  const tokens: CodeToken[] = [];

  for (const line of engine.codeToTokensBase(source, { lang: language, theme: THEME_NAME })) {
    for (const token of line) {
      const kind = KIND_COLOR.exec(token.color ?? "")?.[1] ?? "";

      if (!isTokenKind(kind) || token.content === "") {
        continue;
      }

      const previous = tokens.at(-1);

      if (previous?.kind === kind && previous.offset + previous.length === token.offset) {
        tokens[tokens.length - 1] = { ...previous, length: previous.length + token.content.length };
      } else {
        tokens.push({ offset: token.offset, length: token.content.length, kind });
      }
    }
  }

  return { language, tokens };
}
