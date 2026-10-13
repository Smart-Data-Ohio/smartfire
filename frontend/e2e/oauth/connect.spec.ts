import { createHash, randomBytes } from "node:crypto";
import { expect, test } from "@playwright/test";

const issuer = "https://localhost:4398";

const clientId = "https://client.example:4398/client.json";

const callback = "https://client.example:4398/callback";

const resource = `${issuer}/agents/mcp`;

const password = "secret123456";

test("a signed-out member connects a CIMD client with PKCE and uses MCP", async ({
  page,
  request,
}) => {
  const verifier = randomBytes(32).toString("base64url");
  const state = randomBytes(16).toString("hex");
  const challenge = createHash("sha256").update(verifier).digest("base64url");

  // The client handles its callback itself and exchanges the code without the member's cookies.
  await page.route(`${callback}?*`, async (route) => {
    const landed = new URL(route.request().url());

    expect(landed.searchParams.get("state")).toBe(state);
    expect(landed.searchParams.get("iss")).toBe(issuer);
    expect(landed.searchParams.get("error")).toBeNull();

    const code = landed.searchParams.get("code");

    expect(code).toBeTruthy();

    if (!code) throw new Error("The callback has no authorization code");

    const token = await request.post(`${issuer}/oauth/token`, {
      form: {
        grant_type: "authorization_code",
        client_id: clientId,
        redirect_uri: callback,
        resource,
        code,
        code_verifier: verifier,
      },
    });

    expect(token.status()).toBe(200);

    const body = await token.json();

    expect(body.token_type).toBe("Bearer");
    expect(body.scope).toBe("agent");
    expect(body.expires_in).toBe(3600);
    expect(body.refresh_token).toEqual(expect.any(String));

    const mcp = await request.post(resource, {
      headers: { Authorization: [body.token_type, body.access_token].join(" ") },
      data: {
        jsonrpc: "2.0",
        id: 1,
        method: "tools/call",
        params: { name: "list_rooms", arguments: {} },
      },
    });

    expect(mcp.status()).toBe(200);

    const answer = await mcp.json();

    expect(answer.error).toBeUndefined();
    expect(answer.result.isError).not.toBe(true);
    expect(answer.result.structuredContent).toEqual([expect.objectContaining({ name: "HQ" })]);
    await route.fulfill({
      contentType: "text/html",
      body: "<h1>Client connected and read HQ</h1>",
    });
  });

  const authorize = new URL(`${issuer}/oauth/authorize`);

  authorize.search = new URLSearchParams({
    client_id: clientId,
    redirect_uri: callback,
    response_type: "code",
    scope: "agent",
    resource,
    state,
    code_challenge: challenge,
    code_challenge_method: "S256",
  }).toString();
  await page.goto(authorize.href);
  await expect(page).toHaveURL(/\/app\/session\/new/);
  await page.getByLabel("Email address", { exact: true }).fill("kevin@37signals.com");
  await page.getByLabel("Password", { exact: true }).fill(password);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();

  // The frozen seed's member has two-step sign-in enrolled.
  const code = process.env.SMARTFIRE_E2E_TOTP;

  if (!code) throw new Error("Run this spec through the frozen-seed Rust acceptance test");

  await page.getByLabel("Authenticator code", { exact: true }).fill(code);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();

  await expect(
    page.getByRole("heading", { name: "S9 test client wants to connect as one of your agents" }),
  ).toBeVisible();
  await expect(page).toHaveURL(/\/app\/oauth\/consent\//);
  await expect(page.getByText(/hasn't verified this app/)).toBeVisible();
  await page.getByLabel("Name", { exact: true }).fill("S9 helper");
  await page.getByRole("checkbox", { name: "HQ", exact: true }).check();
  await page.getByRole("button", { name: "Approve", exact: true }).click();

  const confirmation = page.getByRole("dialog", { name: "Confirm it's you" });

  await expect(confirmation).toBeVisible();
  await confirmation.getByLabel("Password", { exact: true }).fill(password);
  await confirmation.getByLabel("Password", { exact: true }).press("Enter");
  await expect(page.getByRole("heading", { name: "Client connected and read HQ" })).toBeVisible();
  expect(new URL(page.url()).origin + new URL(page.url()).pathname).toBe(callback);
});
