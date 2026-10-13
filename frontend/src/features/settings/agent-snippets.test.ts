import { describe, expect, it } from "vitest";
import { agentSnippet, serverName, shellWord, tokenVariable } from "./agent-snippets.ts";

const SECRET = `sfa_${"0123456789abcdef".repeat(4)}`;

const INPUT = {
  agentName: "My Claude",
  mcpUrl: "https://chat.example.com/agents/mcp",
  secret: SECRET,
};

describe("names", () => {
  it("names the server after the agent, in lowercase words joined by dashes", () => {
    expect(serverName("My Claude")).toBe("smartfire-my-claude");
    expect(serverName("  Ünïcödé — bot!! ")).toBe("smartfire-unicode-bot");
    expect(serverName("🤖")).toBe("smartfire");
    expect(serverName("x".repeat(60))).toBe(`smartfire-${"x".repeat(40)}`);
  });

  it("names the token's variable after the server", () => {
    expect(tokenVariable("My Claude")).toBe("SMARTFIRE_MY_CLAUDE_TOKEN");
    expect(tokenVariable("")).toBe("SMARTFIRE_TOKEN");
  });

  it("quotes a shell word only when it needs it", () => {
    expect(shellWord("https://a.example/agents/mcp")).toBe("https://a.example/agents/mcp");
    expect(shellWord("a b")).toBe("'a b'");
    expect(shellWord("it's")).toBe(`'it'\\''s'`);
  });
});

describe("agentSnippet", () => {
  it("adds Claude Code's HTTP server with the token in a header, for every project", () => {
    const snippet = agentSnippet("claude-code", INPUT);

    expect(snippet.steps).toHaveLength(1);
    expect(snippet.steps[0]?.code).toBe(
      [
        "claude mcp add --transport http --scope user smartfire-my-claude https://chat.example.com/agents/mcp \\",
        `  --header 'Authorization: Bearer ${SECRET}'`,
      ].join("\n"),
    );
  });

  it("points Codex at a variable that holds the token", () => {
    const [config, variable] = agentSnippet("codex", INPUT).steps;

    expect(config?.label).toBe("Add to ~/.codex/config.toml");
    expect(config?.code).toBe(
      [
        "[mcp_servers.smartfire-my-claude]",
        'url = "https://chat.example.com/agents/mcp"',
        'bearer_token_env_var = "SMARTFIRE_MY_CLAUDE_TOKEN"',
      ].join("\n"),
    );
    expect(config?.code).not.toContain(SECRET);
    expect(variable?.code).toBe(`export SMARTFIRE_MY_CLAUDE_TOKEN=${SECRET}`);
  });

  it("gives Cursor an mcp.json entry whose header reads the variable", () => {
    const [config, variable] = agentSnippet("cursor", INPUT).steps;

    expect(config?.label).toBe("Add to ~/.cursor/mcp.json");
    expect(JSON.parse(config?.code ?? "")).toEqual({
      mcpServers: {
        "smartfire-my-claude": {
          url: "https://chat.example.com/agents/mcp",
          // biome-ignore lint/suspicious/noTemplateCurlyInString: Cursor's own ${env:NAME} syntax, kept literal
          headers: { Authorization: "Bearer ${env:SMARTFIRE_MY_CLAUDE_TOKEN}" },
        },
      },
    });
    expect(config?.code).not.toContain(SECRET);
    expect(variable?.code).toBe(`export SMARTFIRE_MY_CLAUDE_TOKEN=${SECRET}`);
  });

  it("gives any other client the URL, the header and an API tool entry", () => {
    const [url, header, tool] = agentSnippet("other", INPUT).steps;

    expect(url?.code).toBe("https://chat.example.com/agents/mcp");
    expect(header?.code).toBe(`Authorization: Bearer ${SECRET}`);
    expect(JSON.parse(tool?.code ?? "")).toEqual({
      type: "mcp",
      server_label: "smartfire-my-claude",
      server_url: "https://chat.example.com/agents/mcp",
      authorization: SECRET,
    });
  });

  it("gives a web connector its URL and steps, and no token", () => {
    for (const client of ["claude-ai", "chatgpt", "grok"] as const) {
      const snippet = agentSnippet(client, INPUT);

      expect(snippet.steps).toEqual([
        { label: "Connector URL", code: "https://chat.example.com/agents/mcp" },
      ]);
      expect(snippet.note).toContain("sign in to Smartfire");
    }
  });

  it("quotes a URL the shell would split", () => {
    const snippet = agentSnippet("claude-code", { ...INPUT, mcpUrl: "https://x.example/a b" });

    expect(snippet.steps[0]?.code).toContain("smartfire-my-claude 'https://x.example/a b' \\");
  });

  it("leaves a placeholder where no token is shown", () => {
    const snippet = agentSnippet("codex", { agentName: "A", mcpUrl: INPUT.mcpUrl });

    expect(snippet.steps[1]?.code).toBe("export SMARTFIRE_A_TOKEN=YOUR_TOKEN");
  });
});
