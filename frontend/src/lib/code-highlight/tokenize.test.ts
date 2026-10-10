// @vitest-environment node

import { describe, expect, it } from "vitest";
import { detectLanguage } from "./detect.ts";
import { TOKEN_KINDS } from "./kinds.ts";
import { highlight, LANGUAGE_ALIASES, resolveLanguage } from "./tokenize.ts";

/** Classic's samples (web/script/code-highlighter/highlighter.test.mjs), one per grammar. */
const SAMPLES: ReadonlyMap<string, string> = new Map([
  ["bash", 'echo "$HOME"'],
  ["c", "int main(void) { return 0; }"],
  ["cpp", "template<typename T> class Example {};"],
  ["csharp", 'public class Example { public string Name => "Sam"; }'],
  ["css", "body { color: red; }"],
  ["diff", "@@ -1 +1 @@\n-old\n+new"],
  ["dockerfile", "FROM ruby:3.4"],
  ["go", "func main() { return }"],
  ["html", '<img onerror="alert(1)">'],
  ["java", "public class Example { public int value = 1; }"],
  ["javascript", "const value = true;"],
  ["json", '{"value": true}'],
  ["jsx", "const App = () => <div>Hello</div>;"],
  ["powershell", 'Write-Host "Hello"'],
  ["python", 'def greet(name): return "Hello " + name'],
  ["ruby", "def greet(name)\n  puts name\nend"],
  ["rust", "fn main() { let value = true; }"],
  ["sql", "SELECT name FROM users;"],
  ["typescript", 'const value: string = "hello";'],
  ["tsx", "const App = () => <div>Hello</div>;"],
  ["xml", '<root id="1">Hello</root>'],
  ["yaml", "enabled: true"],
  // The grammars classic's bundle carries in with ruby, html and cpp.
  ["lua", 'local name = "Sam"'],
  ["graphql", "query { user(id: 1) { name } }"],
  ["regex", "^(\\d+)-[a-z]+$"],
  ["glsl", "void main() { gl_FragColor = vec4(1.0); }"],
  ["haml", "%p.greeting= user.name"],
]);

/** Each run's text, checked against the source it claims to cover. */
function runs(source: string, tokens: Awaited<ReturnType<typeof highlight>>["tokens"]): string[] {
  let end = 0;

  return tokens.map((token) => {
    expect(token.offset).toBeGreaterThanOrEqual(end);
    expect(TOKEN_KINDS).toContain(token.kind);
    end = token.offset + token.length;

    return source.slice(token.offset, end);
  });
}

describe("highlight", () => {
  it.each([...SAMPLES])(
    "colours %s in two colours or more at exact offsets",
    async (label, source) => {
      const { tokens } = await highlight(source, label);
      const coloured = runs(source, tokens).join("");

      // Classic's check: more than one colour, the block's plain text colour included.
      expect(coloured.length).toBeGreaterThan(0);
      expect(
        new Set(tokens.map((token) => token.kind)).size + (coloured.length < source.length ? 1 : 0),
      ).toBeGreaterThan(1);
    },
  );

  it("paints Dark+'s keyword, type and string scopes as their kinds", async () => {
    const source = 'const value: string = "hello";';
    const { tokens } = await highlight(source, "ts");

    const kinds = new Map(
      tokens.map((token) => [source.slice(token.offset, token.offset + token.length), token.kind]),
    );

    expect(kinds.get("const")).toBe("keyword");
    expect(kinds.get("string")).toBe("type");
    expect(kinds.get('"hello"')).toBe("string");
  });

  it("paints a diff's added and removed lines as such", async () => {
    const source = "-old\n+new";
    const { tokens } = await highlight(source, "diff");

    expect(tokens.map((token) => token.kind)).toEqual(["deleted", "inserted"]);
  });

  it("keeps tabs, CRLF, blank lines and Unicode exact", async () => {
    const source = '\tconst emoji = "🔥";\r\n\r\n// café\r\n';
    const { tokens } = await highlight(source, "javascript");

    expect(runs(source, tokens)).toContain('"🔥"');
    expect(runs(source, tokens)).toContain("// café");
  });

  it("returns only offsets, so markup in the source is never passed through as HTML", async () => {
    const source = "<script>alert(1)</script>";
    const result = await highlight(source, "html");

    expect(runs(source, result.tokens).join("")).not.toContain("&lt;");
    expect(JSON.stringify(result)).not.toContain("<script>");
  });
});

describe("language detection", () => {
  it("resolves classic's aliases, case-insensitively, to their exact grammar", async () => {
    for (const [alias, expected] of [
      ["c#", "csharp"],
      ["c++", "cpp"],
      ["TS", "typescript"],
      ["tsx", "tsx"],
      ["jsx", "jsx"],
      ["yml", "yaml"],
      ["sh", "shellscript"],
      ["Dockerfile", "docker"],
      ["ps1", "powershell"],
      ["gql", "graphql"],
    ] as const) {
      expect((await highlight("const value = true;", alias)).language, alias).toBe(expected);
    }
  });

  it("knows the same fifty-one labels classic's bundle colours", () => {
    expect(LANGUAGE_ALIASES.size).toBe(51);
    expect(new Set(LANGUAGE_ALIASES.values()).size).toBe(28);
  });

  it("guesses an unlabelled block with Highlight.js, as classic does", async () => {
    expect(detectLanguage(SAMPLES.get("python") ?? "")).toBe("python");
    expect(await resolveLanguage(SAMPLES.get("python") ?? "", undefined)).toBe("python");
    expect(await resolveLanguage("FROM ruby:3.4\nRUN bundle install", undefined)).toBe("docker");
    expect((await highlight(SAMPLES.get("python") ?? "", undefined)).tokens.length).toBeGreaterThan(
      0,
    );
  });

  it("leaves unknown labels and text fences plain, never guessing", async () => {
    for (const label of ["text", "txt", "plaintext", "unknown", "__proto__", "constructor"]) {
      expect(await highlight("const value = true;", label), label).toEqual({
        language: "text",
        tokens: [],
      });
    }
  });

  it("leaves prose Highlight.js calls plaintext uncoloured", async () => {
    expect(await highlight("hello there", undefined)).toEqual({ language: "text", tokens: [] });
  });
});
