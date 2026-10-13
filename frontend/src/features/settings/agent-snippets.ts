/**
 * How each client connects to a personal agent's MCP server with a token: the command or config
 * the Connect step shows beside the secret. The syntax follows each client's own docs (checked
 * 2026-10-13): Claude Code's `claude mcp add --transport http … --header` (code.claude.com/docs/en/mcp),
 * Codex's `[mcp_servers.<name>]` table with `bearer_token_env_var` in `~/.codex/config.toml`, and
 * Cursor's `mcpServers` in `mcp.json` with `headers` and `${env:NAME}` interpolation
 * (cursor.com/docs/mcp). Without a token, with the server's OAuth: Claude Code's `/mcp` →
 * Authenticate after `claude mcp add` with no header, and Codex's `codex mcp login <name>` for a
 * `url`-only server.
 */

/** The clients that connect with a token. */
export type TokenClient = "claude-code" | "codex" | "cursor" | "other";

/** The web apps that connect through OAuth, offered only when the server has it. */
export type WebConnector = "claude-ai" | "chatgpt" | "grok";

export type AgentClient = TokenClient | WebConnector;

/** The terminal clients that can also sign in through OAuth instead of holding a token. */
export type SignInClient = "claude-code" | "codex";

/** Whether `client` can sign in through OAuth instead of holding a token. */
export function canSignIn(client: AgentClient | null): client is SignInClient {
  return client === "claude-code" || client === "codex";
}

/** One thing to copy, and what to do with it. */
export interface SnippetStep {
  readonly label: string;
  readonly code: string;
}

export interface Snippet {
  readonly steps: readonly SnippetStep[];
  /** A line under the steps: where the setting lives, or what to do next. */
  readonly note: string;
}

export interface SnippetInput {
  /** The agent's name, for the server's name in the client's config. */
  readonly agentName: string;
  /** `/agents/mcp` on this server (`PersonalAgentList.mcpUrl`). */
  readonly mcpUrl: string;
  /** The token, shown once. Absent for a web connector, which signs in instead. */
  readonly secret?: string;
}

/**
 * The name the client lists the server under: `smartfire-` and the agent's name in lowercase
 * letters, digits and dashes (a bare TOML key, a JSON key, and a `claude mcp` name alike).
 */
export function serverName(agentName: string): string {
  const slug = agentName
    .normalize("NFKD")
    .replace(/\p{M}/gu, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 40)
    .replace(/-+$/, "");

  return slug === "" ? "smartfire" : `smartfire-${slug}`;
}

/** The environment variable that holds the token: `SMARTFIRE_<NAME>_TOKEN`. */
export function tokenVariable(agentName: string): string {
  const name = serverName(agentName).replace(/-/g, "_").toUpperCase();

  return `${name}_TOKEN`;
}

/** `value` as one POSIX shell word: bare when it is plainly safe, else single-quoted. */
export function shellWord(value: string): string {
  return /^[A-Za-z0-9_@%+=:,./-]+$/.test(value) ? value : `'${value.replaceAll("'", `'\\''`)}'`;
}

/** The secret, or a placeholder where the client needs it and none is shown. */
function tokenOf(input: SnippetInput): string {
  return input.secret ?? "YOUR_TOKEN";
}

function exportLine(variable: string, token: string): string {
  return `export ${variable}=${shellWord(token)}`;
}

function claudeCode(input: SnippetInput): Snippet {
  const header = `Authorization: Bearer ${tokenOf(input)}`;

  return {
    steps: [
      {
        label: "Run in a terminal",
        code: [
          `claude mcp add --transport http --scope user ${shellWord(serverName(input.agentName))} ${shellWord(input.mcpUrl)} \\`,
          `  --header ${shellWord(header)}`,
        ].join("\n"),
      },
    ],
    note: "--scope user makes it available in every project. Check it with claude mcp list.",
  };
}

function codex(input: SnippetInput): Snippet {
  const variable = tokenVariable(input.agentName);

  return {
    steps: [
      {
        label: "Add to ~/.codex/config.toml",
        code: [
          `[mcp_servers.${serverName(input.agentName)}]`,
          `url = ${JSON.stringify(input.mcpUrl)}`,
          `bearer_token_env_var = ${JSON.stringify(variable)}`,
        ].join("\n"),
      },
      {
        label: "Set the token in your shell profile",
        code: exportLine(variable, tokenOf(input)),
      },
    ],
    note: "Codex sends the variable as the Authorization header. Start a new shell, then run codex.",
  };
}

function cursor(input: SnippetInput): Snippet {
  const variable = tokenVariable(input.agentName);

  const config = {
    mcpServers: {
      [serverName(input.agentName)]: {
        url: input.mcpUrl,
        headers: { Authorization: `Bearer \${env:${variable}}` },
      },
    },
  };

  return {
    steps: [
      { label: "Add to ~/.cursor/mcp.json", code: JSON.stringify(config, null, 2) },
      {
        label: "Set the token where Cursor starts",
        code: exportLine(variable, tokenOf(input)),
      },
    ],
    note: "Restart Cursor after setting the variable, then turn the server on in its MCP settings.",
  };
}

function other(input: SnippetInput): Snippet {
  const tool = {
    type: "mcp",
    server_label: serverName(input.agentName),
    server_url: input.mcpUrl,
    authorization: tokenOf(input),
  };

  return {
    steps: [
      { label: "Server URL (Streamable HTTP)", code: input.mcpUrl },
      { label: "Header", code: `Authorization: Bearer ${tokenOf(input)}` },
      { label: "As an MCP tool in an API request", code: JSON.stringify(tool, null, 2) },
    ],
    note: "Any MCP client that can send a header works. The token is a bearer token.",
  };
}

const CONNECTOR_SETTINGS: Readonly<Record<WebConnector, string>> = {
  "claude-ai": "In claude.ai, open Settings → Connectors and add a custom connector",
  chatgpt: "In ChatGPT, open Settings → Apps & Connectors and create a connector",
  grok: "In Grok, open Settings → Connectors and add a custom connector",
};

function webConnector(client: WebConnector, input: SnippetInput): Snippet {
  return {
    steps: [{ label: "Connector URL", code: input.mcpUrl }],
    note: `${CONNECTOR_SETTINGS[client]} with this URL, then sign in to Smartfire and approve it.`,
  };
}

/**
 * How a terminal client connects without a token: added with only the URL, then signed in from
 * the client, which opens Smartfire's approval page in the browser.
 */
export function signInSnippet(client: SignInClient, mcpUrl: string): Snippet {
  const name = serverName("");

  if (client === "claude-code") {
    return {
      steps: [
        {
          label: "Run in a terminal",
          code: `claude mcp add --transport http --scope user ${name} ${shellWord(mcpUrl)}`,
        },
        { label: "Then, in Claude Code", code: "/mcp" },
      ],
      note: `Choose ${name} and Authenticate. Your browser opens Smartfire, where you choose the agent and approve it.`,
    };
  }

  return {
    steps: [
      {
        label: "Add to ~/.codex/config.toml",
        code: [`[mcp_servers.${name}]`, `url = ${JSON.stringify(mcpUrl)}`].join("\n"),
      },
      { label: "Then sign in", code: `codex mcp login ${name}` },
    ],
    note: "Your browser opens Smartfire, where you choose the agent and approve it.",
  };
}

/** What `client` needs to reach the agent: the steps the Connect step shows. */
export function agentSnippet(client: AgentClient, input: SnippetInput): Snippet {
  switch (client) {
    case "claude-code":
      return claudeCode(input);

    case "codex":
      return codex(input);

    case "cursor":
      return cursor(input);

    case "other":
      return other(input);

    case "claude-ai":
    case "chatgpt":
    case "grok":
      return webConnector(client, input);
  }
}
